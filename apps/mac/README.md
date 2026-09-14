# FlashTeX Mac shell (FT-003)

Native macOS source editor + preview shell. Swift Package, macOS 14+, SwiftUI/AppKit.
Owner: mac-claude-a. Contract: `docs/contracts/runtime-v1.md`.

The preview is **fixture-backed**: it renders `protocol/fixtures/compile-result.json`
and shows a `FIXTURE` badge, result id/revision/status, and `pdf: none`. No LaTeX is
compiled; when the buffer is edited the banner says the preview was not recompiled.

## Build and test

```sh
cd apps/mac
swift build
swift test
xcodebuild -scheme FlashTeXMac -destination 'platform=macOS' build
FLASHTEX_REPO=$(git rev-parse --show-toplevel) .build/debug/FlashTeXMac
```

The app locates `protocol/fixtures/` via `FLASHTEX_REPO`, the working directory,
the bundle path, or the source path; `File > Open Compile Result Fixture…` (⌘O)
loads another `compile_result` JSON, with a sibling `compile-request.json` (or
`<name>-request.json` for a `<name>-result.json`) used to seed the editor when
present. `File > Reload Fixture` (developer-only, no shortcut) reloads it; loading
a fixture over a real document confirms first and detaches the file, so Save can
never overwrite it (#72).

## Packaging

A bare `swift build` product has no Finder/Dock identity: `open -a` fails on it
and macOS cannot grant it per-app permissions (e.g. local network, later).
`scripts/make-app.sh` wraps the built executable in a minimal `FlashTeX.app`:

```sh
apps/mac/scripts/make-app.sh [--debug] [--compiler <path>] [--pdf <path>] [--open] [--install] [--dmg]
```

It builds `FlashTeXMac` (release by default), assembles
`apps/mac/build/FlashTeX.app` (`CFBundleIdentifier tech.jay3332.flashtex.mac`),
copies `protocol/fixtures/compile-{request,result}.json` and `Samples/*` into
`Contents/Resources/Samples` (so a bundled app finds fixtures via
`Bundle.main.resourceURL`), bundles `flashtex-compiler`/`flashtex-pdf` into
`Contents/MacOS` when built at `crates/{compiler,pdf}/target/release/…` under
the repo root (or passed via `--compiler`/`--pdf`), and ad-hoc codesigns the
result. Launch with `open apps/mac/build/FlashTeX.app` or pass `--open`.
`apps/mac/build/` is gitignored. `--install` atomically replaces
`~/Applications/FlashTeX.app` (verified to launch before the previous bundle
is discarded) and `--dmg` produces a compressed disk image; see
`apps/mac/docs/packaging.md` for signing/notarization status and the
update-path and launch-recovery evidence (`scripts/launch-check.sh`).

### Rooted TeX metrics for no-TeX operation (GH36)

`flashtex-render` lays text out with Latin Modern's TeX metrics. It looks for
`.tfm` files in `FLASHTEX_TFM_DIRS` (colon separated) before anything it infers,
and loads its digest-bound required 12 pt set only from a rooted `texmf` tree:
the directory must end in `fonts/tfm/public/lm` and
`<root>/doc/fonts/lm/GUST-FONT-LICENSE.TXT` must sit beside it. A flat
`Resources/Fonts` cannot satisfy that, and a MacTeX on the build machine can
hide the gap. The bundle therefore ships the five official LM 2.004 metrics the
established fixtures need (`ec-lmr10`, `ec-lmr12`, `rm-lmr12`, `rm-lmr8`,
`rm-lmr6`) plus the rooted license, vendored under `apps/mac/Fonts/texmf/…`:

- `scripts/bundle-texmf.py` (called by `make-app.sh`) verifies every file's
  byte length and SHA-256 against the Commander's pinned manifest
  (`crates/rendering-core/docs/handoffs/native-assets/manifest.json`, itself
  SHA-pinned in `crates/rendering-core/tools/verify_bundle_resources.py`)
  BEFORE the build, stages them into `Contents/Resources/texmf/fonts/tfm/public/lm`
  and `Contents/Resources/texmf/doc/fonts/lm/GUST-FONT-LICENSE.TXT`, then runs
  the pinned verifier over the whole `Resources` directory (3 fonts + 5 metrics
  + license) before signing. Any missing/mismatched/symlinked file refuses
  packaging; nothing is downloaded and the host TeX tree is never consulted.
  `FLASHTEX_BUNDLE_TEXMF_ROOT` points at another verified official root.
- The verified hashes are recorded in `Contents/Resources/components.json`
  (`"resources"`) and `Contents/Resources/resource-coverage.json`, both sealed by
  the app signature.
- `BundledMetrics` (`BundledMetrics.swift`) finds the bundled directory (bundle
  `Resources/texmf`, else the repository copy for `swift build` products) and
  appends it to `FLASHTEX_TFM_DIRS` for every producer launch — the directly
  attached worker (`WorkerClient`) and the helper-spawned producer
  (`PreviewControllerClient` → `flashtex-preview-controller` → compiler child,
  which inherits the helper's environment). Policy: explicit user entries come
  first and override the bundle (a populated user directory wins — verified by
  `BundledMetricsTests` with a truncated user `ec-lmr10.tfm`, which the producer
  then reads and reports as `tfm_missing … InvalidFont` although an intact copy
  sits behind it); the bundled directory is the fallback for everything the user
  entries do not carry; nothing else in the environment changes.
  A producer at or after render-pipeline 421a2049 also discovers
  `<exe>/../Resources/texmf` by itself (explicit env first, then the bundle,
  then host TeX); the env route keeps older producers and explicit overrides
  working and is what the shell sets regardless.
- Supplementary metrics (`apps/mac/Fonts/texmf/SUPPLEMENTARY-METRICS.json`):
  23 further Latin Modern text TFMs — `ec-lmr{5,6,7,8,9,17}`,
  `ec-lmbx{5,6,7,8,9,10,12}`, `ec-lmri{7,8,9,10,12}`, `ec-lmbxi10`,
  `rm-lmr{5,7,9,10}` — so 5–17 pt regular, 5–12 pt bold, 7–12 pt italic,
  10 pt bold-italic and 5–10 pt roman math lay out with TeX metrics (GH34:
  10/11 pt documents are `ok`, not `recovered/tfm_missing`). They are NOT in
  the Commander's manifest: copied from MacTeX 2026 (TeX Live `lm` rev 77682,
  catalogue 2.005, MANIFEST 2.004) and byte-identical to the CTAN `lm.zip`
  copy on the build machine, but not verified against the pinned 2.004
  archive; their SHA-256/lengths are pinned in that JSON, verified by
  `bundle-texmf.py` on every package (drift refuses packaging), and recorded
  under `components.json` `resources.supplementary`. Not covered: sans,
  typewriter, caps, slanted, dunhill; the `lmmi/lmsy/lmex` math families come
  from Latin Modern Math (OTF).
- Acceptance: `scripts/texmf-acceptance.sh [--app …] [--evidence <dir>]` runs
  the producer actually inside the bundle with host TeX excluded (`env -i`,
  `PATH=/usr/bin:/bin`, empty `HOME`, no `FLASHTEX_*`/`TEXMF*`, plus a
  `sandbox-exec` profile denying reads under `/usr/local/texlive`,
  `/Library/TeX`, `/usr/share/texmf|texlive`) on the corrected 10 pt
  multi-document request and 12 pt text/math documents through the direct route,
  the route with a user entry appended, and the bundled preview controller;
  requires zero `tfm_missing`/`required_metrics_unavailable`/`font_unavailable`
  diagnostics, an explicit failure once `ec-lmr10.tfm` is deleted from a copy,
  and verifier exit 0. `packaging-selftest.sh` covers the refusal paths without
  a build and runs the acceptance in `--full` mode; `launch-check.sh` verifies
  the resources statically. Evidence: `docs/evidence/mac-bundle-texmf-<UTC>/`
  (134120Z: env route with an unpatched f762f82a producer; 135157Z and
  135718Z: discovery + env routes with producers 98e829bf and 9aaec57a).
  `make-app.sh --source-sha render=<sha>` records the producer's source
  revision in `components.json` (`git_sha_origin: declared`) when the binary
  was built outside a repository checkout.

## Behavior

- Editor: `NSTextView` (monospaced, undo, no smart substitutions). Footer shows
  UTF-8 byte and UTF-16 unit counts of the active document.
- Projects from scratch (`ProjectScaffold.swift`, `ProjectScaffoldViews.swift`):
  *File › New Project…* (⌘⌥N) writes a template (`ProjectTemplate`: Blank article,
  Article with sections via `\input`, Report with chapters via `\include`,
  Homework sheet with the HW1-style preamble and `\problem`) into
  `<folder>/<name>/` and opens `main.tex` through `openTex` — existing template
  files are refused unless confirmed. *New File…* (⌘N, sidebar +, project-row
  context menu) resolves a rooted `.tex` name (`NewFilePath`: subfolders yes,
  `..`/absolute no, `main.tex` no), writes it under `ProjectDocuments.rootedFile`,
  opens it via `openDocument`, and optionally posts `\input{name}` at the caret
  as one `pendingEdit`. A literal `\input`/`\include` with no file is a
  "missing — create" sidebar row, and the compiler's `included file not found:
  looked for 'x' and 'x.tex'` diagnostic gets a **Create x.tex** button on its
  Problems row (`MissingIncludeFix`, deterministic message parse). Context-menu
  *Rename…* moves the file, retargets the member's metadata and rewrites
  references in open documents (`ReferenceRewrite.plan`: one grouped
  `pendingEdit` per document, applied in sequence as the editor consumes each);
  *Delete…* detaches and `FileManager.trashItem`s. Both refuse the entry
  document and unsaved edits. Tests: `ProjectScaffoldTests`.
- Preview: pages drawn at 1pt = 1 screen point, origin top-left; text items are
  placed by `x_pt` / `baseline_y_pt` / `font_size_pt`. Hover highlights an item;
  clicking it navigates to its `source` range. Diagnostics list with "Go to source".
- Navigation converts the contract's zero-based end-exclusive UTF-8 byte offsets to
  a UTF-16 `NSRange` (`String.nsRange(utf8Bytes:)`). Offsets that are out of range,
  reversed, or inside a multi-byte scalar are rejected with a footer message rather
  than applied. Unknown item `kind`s decode as `.unknown`; on the legacy route
  they are skipped, on a negotiated route they are reported (see "Negotiated
  layout capabilities").
- Caret sync (source→preview): text items whose `source` range contains the
  editor caret (UTF-16 caret → UTF-8 byte via `String.utf8ByteRange(of:)`,
  `CaretSync.itemsContaining`) get a secondary highlight (15% accent fill +
  underline) on every page. Empty ranges match only an exactly equal byte. The
  preview does not auto-scroll to the highlighted item.
- Inline diagnostics: each diagnostic with a `source` in the active document is
  underlined in the editor (red dotted for errors, orange for warnings); hovering
  shows the message and, when present, the `recovery` text (`EditorDiagnostics`).
  Marks are layout-manager temporary attributes, so undo and the text binding are
  unaffected. After edits a mark is rebased through `SourceMapping` or dropped when
  it overlaps the edited region — never drawn under the wrong text. Diagnostics
  with null `source` appear only in the preview's diagnostics list.
  Partial output (`recovered` with pages AND diagnostics) marks every reported
  span, including spans inside regions the compiler skipped; a diagnostic raised
  while expanding a user macro is reported at the macro's call site (HW1:
  `\problem` carries `\subsection`/`\hfill`/`\normalfont`, `\Z` carries
  `\mathbb`), verified for all 119 HW1 diagnostics in
  `EditorDiagnosticsPartialOutputTests`. A `failed` result with no pages keeps
  the last result's underlines, rebased and flagged "kept from revision N:
  revision M failed with no output" (tooltip, VoiceOver line, footer) — never
  cleared, never duplicated across consecutive failures; any result with
  output replaces them (`EditorDiagnostics.Retained`,
  `ShellModel+DiagnosticRetention.swift`). The diagnostics list groups
  identical diagnostics (same severity and message) into one row — "12× `\in`
  is not supported in math mode" — with an "N places" menu that jumps to each
  occurrence ("3 of 12: main.tex line 41"); Fix… and the explanation line
  belong to the first occurrence (`EditorDiagnostics.groups`). The panel
  (`DiagnosticsListView`, DiagnosticsPanel.swift) has a keyboard selection:
  ↑/↓ pick a row, Return jumps to the row's current occurrence, Esc gives the
  keyboard back to the editor at its caret, ⌘⌥] / ⌘⌥[ (Navigate) step through
  the selected group's places (wrapping) and the row — and its VoiceOver label
  — then reads "12 places, 3 of 12, main.tex line 41". ⌘C on the focused list
  or Edit > Copy Diagnostics as Text (⌘⌥C) copies `path:line: error: message`
  lines for the selected row (every place; all diagnostics when none is
  selected; `-:0:` for unsourced ones) for pasting into an issue
  (`DiagnosticsPanelTests`).
- Inline math preview on hover (`MathHoverPreview.swift`, `HoverController.mathPreview`):
  resting the pointer over an inline formula (`$…$`, `\(…\)`) shows a small
  popover with the formula as already rendered — cropped straight out of the
  current v2 page bitmap (`V2PageRasterizer.images`), a few points of padding
  added; nothing is rendered on the hover path itself. The formula's span comes
  from `EditorIntelligence.inlineMathSpan` (the enclosing `.mathDelimiter` pair
  from `SyntaxHighlighter`, delimiters included), converted to a UTF-8 byte
  range and matched against every page item carrying that exact `source` span
  (`V2Geometry.formulaBox`, the same machinery the caret's formula-box
  highlight uses). Shows nothing when the preview is stale, when the current
  frame has no item for that span, or when the formula's source spans more
  than two editor lines; a ≤2-line formula's box is the union of every member
  item. Display math (`$$…$$`, `\[…\]`) and math environments are not covered.
  Debounced like other hover (the same 0.45 s timer; nothing while typing).
  Tests: `MathHoverTests`.
- Dark preview toggle in the preview header (page and text colors only).
- Stale offsets are never applied. Each `compile_result` remembers the exact
  document text it was produced for; after edits, a span is rebased through the
  common prefix/suffix of old vs new text (`SourceMapping`), verified against the
  item's text when known, and refused ("recompile to navigate") if it overlaps the
  edited region. Multiple edits collapse conservatively.
- Auto-compile (toolbar toggle, default on when a worker is attached): edits are
  debounced (`FLASHTEX_DEBOUNCE_MS`, default 0) and coalesced — one request in
  flight, the newest buffer goes out when it returns; only a layout-capability
  switch is sent immediately alongside it (see "Negotiated layout capabilities").
  Latency (send→result) is shown in the banner with a median.
  Measured with the FT-002 compiler on an M1 Max: 0.5–9 ms per request; a 10-edit
  burst coalesced into 2 requests (`RealCompilerTests`).
- Worker transport: `File > Attach Built Compiler` (⌘⇧K) finds `$FLASHTEX_COMPILER`
  or `crates/compiler/target/{release,debug}/flashtex-compiler` under the repo;
  `File > Attach Worker Executable…` (⌘K) launches a process
  speaking runtime v1 JSON Lines on stdin/stdout; `Compile` (⌘B) sends the current
  buffers as a `compile` envelope with the editor revision. The banner badge
  switches from `FIXTURE` to `WORKER`; an older `compile_result` never replaces a
  newer one. `error` envelopes, undecodable lines, unsupported versions, and worker
  exit are reported in the banner. Any line over 16 MiB — complete or still
  unterminated — is a protocol violation that terminates the worker, and a worker
  that exits leaving unterminated trailing bytes is reported as a violation too.
  A `compile_result` is applied only if its `id` matches an in-flight request and
  its `project_id` and `revision` match that request; unsolicited or mismatched
  results are logged and never shown.

- Capture review and insertion (contract "Capture and insertion", Mac side):
  `Edit > Pin Insertion Point` (⌘⌥P) records the caret as a `destination_id`
  anchor (UTF-8 byte offset + revision + following context). `Edit > Open Capture
  Proposal…` (no shortcut) queues a `capture_proposal`; a review sheet shows editable
  LaTeX, ambiguities, and required packages. Approve applies exactly one edit
  through the text view's undo manager (⌘Z reverts). Repeated `capture_id`s never
  insert twice. If the buffer changed since pinning, the anchor is rebased by its
  context or, when the destination was deleted/ambiguous, reselection is required.
  See `Samples/capture-proposal.json`. No network or provider call is involved here.
- PDF export: `File > Export PDF…` (⌘⇧E) writes the current preview with
  CoreGraphics/CoreText (`PDFExport.swift`): one PDF page per `pages` entry at
  `width_pt` × `height_pt`, each text item in its resolved face at `font_size_pt`
  with its baseline exactly `baseline_y_pt` from the top (flipped to PDF's
  bottom-left origin), and each typed `rule` item as a filled rectangle
  (`RuleGeometry.pdfRect`). It exports the layout the Rust compiler reported,
  not a TeX-engine PDF: no fonts beyond Latin Modern/Times, no images, no links
  or metadata. The dark toggle only changes page/text colors. Disabled when no
  result is loaded. `File > Print…` (⌘P) prints those same bytes through
  PDFKit's system print panel (`PrintController.swift`); it is also disabled
  when the compile failed or produced no pages (a blank PDF is not printed).
  `File > Print Source…` prints the editor buffer with line numbers from a copy
  and is disabled when no document is open.

## Capture bridge (transfer-v1)

Built against the FT-007 bridge at commit **b5ca96b** on
`agent/commander/capture-bridge` (`crates/bridge`, `docs/contracts/transfer-v1.md`),
an unmerged dependency at the time of writing; no contract changes were made.
The Mac owns UI review, the document transaction and the bridge lifecycle.

`Edit > Attach Capture Bridge` launches `flashtex-bridge --store <dir>`
(`$FLASHTEX_BRIDGE`, a bridge bundled next to the executable, or
`crates/bridge/target/{release,debug}/flashtex-bridge`; the store is
`$FLASHTEX_BRIDGE_STORE` or `~/Library/Application Support/FlashTeX/captures`).
`FLASHTEX_AUTOATTACH=1` or a bundled bridge attaches at launch. The bridge line
under the editor shows attached/error status (error codes as plain text), the
pinned bridge destination, and the latest capture's state.

What works offline (no key, no network — verified with `RealBridgeTests`):

- On attach the shell first reconciles its edit ledger with `capture_status`
  (below), then sends `document_open` for the active document at the current
  editor revision. Every later edit is sent as one `document_edit` (byte range
  + replacement, derived from the common prefix/suffix of old and new text and
  widened to UTF-8 scalar boundaries); a refused edit triggers a `document_open`
  resynchronization. `File > Open` re-opens the new document.
- `Pin Insertion Point` (⌘⌥P) also sends `destination_pin` for the caret (or
  selection) byte range and stores the returned anchor with its immutable
  binding. The local context anchor remains for offline review.
- `Edit > Submit Sample Capture…` (⌘⇧U) sends `capture_submit` for a chosen
  PNG/JPEG (base64) with the pinned `destination_id` and `base_revision` =
  the anchor's pinned revision. `capture_received {durable:true}` is shown;
  identical retries return the same record, a different payload under the same
  ID is `capture_id_conflict`, a non-decodable image is `invalid_image`.
- `Edit > Convert Capture` (⌘⇧G) sends `capture_convert {capture_id,
  supported_features: []}`. Without a conversion provider flag (today the
  bridge's `--enable-grok`) the bridge answers `provider_disabled` (with it but
  no key, `provider_auth_missing`); both are
  shown as text and never prompt for a key. A `capture_proposal` (with
  `context_revision`) is queued in the existing review sheet.
- Approving a bridge proposal sends `capture_prepare_insert {capture_id,
  expected_revision: editorRevision, approved: true}` and verifies the returned
  `capture_edit`: project/path, `expected_revision == editorRevision`, SHA-256 of
  the current UTF-8 buffer == `document_before_sha256` (CryptoKit), scalar-aligned
  `start_byte..end_byte`, and `removed_text` equal to those bytes. The edit is
  then committed **durably first** through the edit-ledger helper (below):
  source and applied edit ID land together in one fsynced record and a receipt
  comes back. Only then is the durable document adopted in the editor as one
  undoable edit (`pendingEdit`), the `.tex` re-exported atomically when the
  document is file-backed, and `capture_applied {capture_id, edit_id,
  new_revision}` sent — no `document_edit` for that change. On the bridge's
  acknowledgement the helper drops its recovery snapshot (`confirm`). A helper
  refusal or persistence failure inserts nothing anywhere and sends no
  receipt; a persistence failure poisons the helper handle, which is relaunched
  and its on-disk state re-read before anything else. The same `edit_id` is
  never applied twice (bridge `already_applied`, helper `edit_id_conflict` /
  original-receipt replay), also after undo. Reviewer-edited LaTeX is refused
  (the contract has no field for it). `Reject` sends `capture_reject`.
- Restart reconciliation (ledger-guarded): before the current document is
  opened, the shell asks the helper for a `recovery_export` (snapshot token +
  pending receipts), queries the bridge `capture_status` for each pending
  capture, and hands the observations back as one `recovery_import`. An exact
  `applied` receipt is confirmed durably by the helper; a `prepared` edit that
  matches yields `replay_receipt`: the shell reopens the retained pre-edit
  snapshot on the bridge, resends `capture_applied`, and confirms only on an
  exact acknowledgement; anything `unavailable` (transport failure, or a bridge
  with no/conflicting record) keeps its evidence — transport failures are
  reported for retry (`Edit > Retry Bridge Reconciliation`), missing or
  conflicting bridge records are held for explicit operator resolution
  (`resolveReconciliation`, the only path that drops a snapshot). If source or
  ledger changed during the bridge round trip the import is refused as stale
  and the export is repeated (bounded). Nothing is ever reapplied to a changed
  document, and the durable document is authoritative: on attach, a store with
  pending receipts whose text differs from the buffer is adopted into the editor
  as one undoable operation; without pending receipts the buffer replaces the
  stored text (`replace_document`).
- Automatic relaunch of the preview-controller helper: an abnormal exit (crash, SIGKILL) relaunches the same executable for the same project after 0.2/1/3 s, at most 3 times per minute, never after a clean exit or an explicit detach. The helper reopens its ledger, so the durable document returns through the normal `document` reply and the buffer is resubmitted only when it differs; the last preview stays on screen meanwhile (`PreviewControllerTests.testKilledHelperIsRelaunchedAndDurableTextSurvives`).
- Automatic relaunch: an abnormal exit of the bridge or the edit-ledger helper is relaunched with the same store after 0.2/1/3 s, at most 3 times per minute per helper, never after a clean exit or detach; the same reconciliation then runs against the live buffer (ledger realigned, pending receipts settled through `recovery_import`, document reopened, pinned destination re-pinned identically or dropped with a note). A capture or receipt in flight at the crash is shown as `uncertain` and settled exactly once; a capture the bridge never acknowledged may be resubmitted with the same ID.

### Edit ledger helper (durable document transaction)

The durable document store is the Commander's Rust `crates/edit-ledger`
(`flashtex-edit-ledger --store <dir>`), pinned at commit
**afb15839f8080f9e86efd6a187e46dfe014ce559** on
`origin/agent/mac-contract-review/edit-ledger` (an unmerged dependency; built
in a scratch worktree for validation). It is located like the other helpers:
`$FLASHTEX_EDIT_LEDGER`, a `flashtex-edit-ledger` bundled next to the
executable, then `crates/edit-ledger/target/{release,debug}/flashtex-edit-ledger`.
One store per document lives under `<captures store>/documents/`, keyed by the
SHA-256 of the file path (`file-…`) — an unsaved buffer gets a fresh
`unsaved-…` store per attach, so its pending receipts are not recoverable
after a restart (save the document to make them so). Without the helper the
bridge still attaches but capture insertion is disabled (fail closed); the
same when its store is corrupt/unreadable (`invalid_store`), in use by another
process (`store_in_use`), or bound to another document.

`Edit > Retry Bridge Receipt` (withheld export/receipt) and `Edit > Retry
Bridge Reconciliation` (incomplete reconciliation) have no shortcuts.

The Swift side (`EditLedgerClient`, `BridgeSession`) keeps only an in-memory
mirror of the helper's document and transactions. Ordinary typing and undo go
through `replace_document` (expected revision + source hash; applied-ID
tombstones survive undo); a refused replace marks the session diverged and
disables application until the next attach reconciles. Editor revisions and
the helper's document revision advance in lockstep; on attach an older store
is aligned upward, never the editor downward. Every helper reply's
`session_id` / `sequence` / `document_revision` / `document_sha256` is
checked: a foreign session or non-increasing sequence is discarded as a stale
observation, and callbacks of a replaced helper process (or a detached bridge
session) never update the model. All pipe I/O (bridge and helper) runs on a
bounded serial background queue (`LineProcessClient`, 32 MiB in flight,
`backpressure` beyond it, 12 MiB per line) — a stalled reader never blocks the
main thread. `recovery_export` / `recovery_import` are used as above;
`compact` is not used yet.

Requested ledger changes: none required for this integration. Observations for
the ledger owner: (1) the helper answers `invalid_request` with `id: null`
when the operation name is unknown, so such a request can only be matched by
timeout; (2) `replace_document` bumps exactly one revision, so aligning an
older store to a newer editor revision takes one round trip per step (bounded
at 10 000 here); a `set_revision`-style alignment would remove that loop.

Implemented in `ConversionCredential.swift` (see `docs/capture-conversion.md`): the Mac's provider-neutral
credential adapter runs the bridge with the selected provider's flag and supplies its key from the Keychain
(`tech.jay3332.flashtex.ai.<provider>`) or the environment. This is the app's only model-backed feature; see
`docs/extensibility.md` (repository root) for how third parties add their own. Not implemented here:
the companion network transport (captures come from
a file picker), compiler validation of proposals before review, and ledger
compaction.

`Tests/FlashTeXMacTests/Fixtures/fake_bridge.py` and `fake_edit_ledger.py` are
stdlib-Python test doubles of the bridge (in-memory journal, deterministic
`\fakecapture{<id>}` proposal, `%stall`/`%error`/`%garbage` directives) and of
the edit-ledger helper (real atomic `document.json`, same validation and error
codes, service metadata, recovery export/import). `RealBridgeTests` runs only
when `FLASHTEX_BRIDGE` points at a built binary; `RealEditLedgerTests` only
when `FLASHTEX_EDIT_LEDGER` does (both with temporary stores). The shared
fixture `protocol/fixtures/capture-submission.json` decodes since main 9da7e48.

## Samples

`Samples/multipage-result.json` + `multipage-request.json` (⌘O on the result):
two pages, nine text items with byte-exact source ranges into a `main.tex` that
contains non-ASCII words ("naïve", "Résumé") so UTF-8 and UTF-16 offsets differ,
plus one error diagnostic with a source range and recovery text and one warning
with null source/recovery. Use it for manual click-to-source, caret-sync, and
diagnostics checks beyond the one-line contract fixture.

- Faces: LaTeX's default is Computer Modern, so the preview and CoreGraphics
  export use **Latin Modern** (GUST FL; registered at launch from `FLASHTEX_LM_DIR`,
  a bundled `Resources/Fonts`, or BasicTeX's `fonts/opentype/public/lm`) when the
  attached producer is the new `flashtex-render` pipeline or `FLASHTEX_PREVIEW_FACE=latin-modern`
  is set; when attached to today's `flashtex-compiler` (Core-14 Times metrics) they
  draw Times-Roman so glyph widths match the positions. Optical masters follow
  LaTeX (lmroman5/7/8/9/10/12/17).
- Opening another file while the buffer is dirty asks Save / Discard / Cancel;
  a discarded buffer stays recoverable for the session (Edit > Restore
  Discarded Buffer). The Rust-writer export runs off the main actor with both
  child pipes drained concurrently (bounded capture, 30 s timeout), so a chatty
  or stuck writer can neither deadlock nor freeze the UI (issue #19).
- Export is always white: dark preview is a viewing mode only. `File > Export
  PDF…` (⌘⇧E) uses CoreGraphics; `File > Export PDF via Rust Writer…` (⌘⌥E) pipes
  the current `compile_result` envelope to the FT-009 `flashtex-pdf --verify`
  binary (`$FLASHTEX_PDF` or `crates/pdf/target/{release,debug}/flashtex-pdf`).
  `RustPDFExportTests` runs only when `FLASHTEX_PDF` is set.
- Diagnostics panel under the preview is never hidden when diagnostics exist; each
  entry shows severity, message, recovery note (or "no provisional rendering"),
  and source bytes; the banner shows error/warning counts and a `recovered` note.

## Nearby companion (proposal nearby-v1)

The everyday surface is the **Captures** inspector (`View > Toggle Captures`,
⌘⇧I, or the toolbar's Captures button; `CaptureInbox.swift`): opening it
starts advertising and attaches the discovered bridge, *Pairing code…* opens
the Nearby window with a code, and every companion capture appears there at
once with its image, instruction and state (received → converting → proposal
ready → inserted), the syntax-coloured proposal and *Insert at caret* /
*Edit* / *Review…* / *Reject*. A companion's `destination_query` with nothing
pinned pins the caret on its behalf (locally and on the bridge, awaited before
the reply; `mac-caret-N` ids), ⌘⌥P overrides; a nearby capture is converted
as soon as the bridge journals it when a provider is enabled. Launch
advertises when a companion is paired. Switches (`=0` off):
`FLASHTEX_CAPTURE_AUTO_CONVERT`, `FLASHTEX_CAPTURE_CARET_DESTINATION`,
`FLASHTEX_CAPTURES_AUTO_ATTACH`, `FLASHTEX_NEARBY_AUTO_ADVERTISE`;
`FLASHTEX_SHOW_CAPTURES=1` opens the inspector at launch (automation).
Tests: `CaptureInboxTests` (fake bridge + fake ledger).

`Edit > Nearby Companion…` (⌘⇧N) opens a window that advertises this Mac to a
paired iPad/iPhone companion and receives its `capture_submit` messages over an
authenticated, encrypted connection. **This is a proposal until the Commander
publishes `docs/contracts/nearby-v1.md`**; the full text, threat model and
companion checklist are in `docs/nearby-v1-proposal.md`. The companion side is
FT-004's. What is implemented here (Mac side only):

- Discovery: Bonjour `_flashtex._tcp`, instance name = the Mac's name, TXT
  `v=1`, `name=<Mac name>`, `fp=<16 hex, SHA-256 of "flashtex-nearby-v1 mac-id"‖salt>`,
  `salt=<32 hex>`. Discovery only; nothing in TXT is trusted.
- Pairing: "Show Pairing Code" displays a 6-digit CSPRNG code for 120 s. Both
  sides derive a bootstrap PSK and `pair_id` with HKDF-SHA256 from
  (code, salt) (`Pairing.derive`; pinned vector in `PairingTests`). The first
  `hello` over that key returns a random 32-byte long-term `pair_psk`; the
  bootstrap key is then dropped. Pairings persist in
  `~/Library/Application Support/FlashTeX/pairs.json` (mode 0600, atomic
  writes, **not the Keychain**); "Forget" removes one and closes its session.
- Transport (`NearbyListener`): Network.framework `NWListener`, TLS **1.2
  only**, cipher suite **`TLS_PSK_WITH_AES_128_GCM_SHA256` (0x00A8)** only,
  PSK identity = `pair_id`, one key per pairing plus the pending bootstrap key,
  **TLS resumption and tickets disabled** (with resumption on, a removed key
  could still resume — caught by the tests). Ephemeral port; the listener is
  rebuilt on the same port when the key table changes and live sessions are
  adopted, not dropped. An unpaired peer fails the handshake: no line is parsed.
- Framing: runtime-v1 JSON Lines like the bridge; 12 MiB per line including
  the newline, oversized complete or unterminated lines get `error
  line_too_long` and a close. First line must be `hello {pair_id,
  companion_name, protocol_version:1, nonce, proof}` where `proof` is
  HMAC-SHA256(PSK, "flashtex-nearby-v1 hello"‖nonce) — needed because
  Network.framework does not say which table PSK a session used. Reply
  `hello_ack {mac_name, nonce, destination, pair_psk?}`; `destination_query` →
  `destination {destination: {destination_id, project_id, path, base_revision} | null}`
  from the pinned anchor (⌘⌥P), so the companion never types IDs.
- Captures: `capture_submit` is validated (ids, MIME, instructions ≤ 4096 B)
  and handed to a `CaptureSink` (`ShellModel+Nearby.swift`). With a capture
  bridge attached the capture is forwarded through `BridgeSession.submit`
  and the bridge's `capture_received` (durable) or `error` code is returned
  to the companion, and it then appears in the bridge capture list for
  `Convert Capture`. Without a bridge, `receiveNearbyCapture` keeps the last
  50 captures in an in-memory `NearbyInbox` and answers `capture_received
  {capture_id, durable:false, has_proposal:false, applied:false}`; identical
  retries are acknowledged again, a different payload for a known id is
  `capture_id_conflict`. `hello_ack.destination` is the bridge's valid anchor
  when attached, else the local pinned anchor. A plaintext (non-TLS) peer —
  the companion's current `NWParameters.tcp` client — fails the handshake,
  is logged once, and nothing it sent is parsed.
- Threat model and gaps (see the proposal): the 6-digit code is ~20 bits and
  the PSK suite has no forward secrecy, so a passive capture of the pairing
  window can be brute-forced offline — the window is short and one code pairs
  one device; no certificate PKI, no cloud relay, no Keychain, no peer-to-peer
  (AWDL), no companion notification beyond `capture_received`. A bundled
  `.app` will need `NSLocalNetworkUsageDescription`/`NSBonjourServices`; the
  bare executable and `swift test` did not prompt on macOS 26.3.

### Reference companion client (`tools/nearby-client`) — owner: mac-nearby-client

`apps/mac/tools/nearby-client` is a standalone Swift package (macOS 13+/iOS 15+,
no dependency on this app or `FlashTeXProtocol`, so FT-004 can copy
`Sources/NearbyClient` verbatim) plus the `nearby-client` CLI. It is the
executable companion side of the proposal: `NearbyBrowser` (Bonjour with TXT,
`fp` validated against `salt`), `NearbyCrypto` (HKDF/HMAC vectors pinned to
`PairingTests`), `NearbyResolver` (resolve `.service` endpoints to host:port
first — a refused TLS-PSK handshake on a service endpoint otherwise never
leaves `.preparing`), `NearbyConnection` (TLS 1.2 / 0x00A8 / resumption off,
JSON Lines, id-correlated replies), `NearbyClient.pair/connect`, `PairFile`
(0600 JSON, **not** the Keychain) and `NearbyCLI`
(`pair | send | status | browse | forget`, `nearby-client help` prints usage
and exit codes).

Bounded disconnect/reconnect (`NearbyReconnector`, actor):

- `ReconnectPolicy`: `maxAttempts` (default 5, per operation), exponential
  backoff `initialDelay` 0.25 s × 2 up to `maxDelay` 4 s with ±20 % jitter,
  `overallDeadline` 60 s, `connectTimeout` 5 s, `requestTimeout` 30 s. The
  schedule is pure (`delay(beforeAttempt:random:)`), so tests pin it.
- Retryable, and only these: `unreachable` (TCP/DNS refused the dial — reported
  at once from `.waiting`, not after the timeout), `closed`, `timeout`,
  `noMatchingMac`/`browseFailed` (the Mac may be restarting; the Bonjour
  endpoint source re-browses by `fp` before every attempt, so a Mac that
  came back on another port is found).
- Retryable with the Mac's receive caps (proposal §4): `too_many_in_flight`
  and `inbox_full` keep the session — the retry first waits until this
  connection has no request awaiting a reply (`waitUntilIdle`, bounded by
  `requestTimeout`; event `waitingForAcks`), then backs off and re-sends the
  same `capture_id` on the same connection; `too_many_sessions` closes — the
  reconnector drops its own session, backs off (time for the owner to close
  older connections) and reconnects. Both count against the same budget.
- Terminal, never retried: `handshakeFailed` / `pair_mismatch` /
  `pairing_expired` (`needsRepair` → re-pair); `image_too_large`,
  `invalid_image`, `unsupported_image`, `revision_mismatch`,
  `capture_id_conflict`, `bad_request` (`needsNewCapture` → build a new
  capture); every other `error` reply, `invalidInput`, `protocolViolation`,
  `overloaded`, `destinationChanged`, and `attemptsExhausted` once the
  budget is spent; a cancelled task is `cancelled`. Before sending, the
  client mirrors the Mac's cheap image checks (`NearbyWire.checkImage`:
  ≤ 8 MiB, signature matches `mime_type`, valid base64, `base_revision` ≥ 0)
  as `invalidInput`; structure stays the Mac's check (`validateImage: false`
  reaches it). An identical retry on one session is acknowledged by the Mac
  without re-delivery.
- `submit(capture)` is at-least-once with the *same* `capture_id` and payload
  (the Mac de-duplicates; a bridge journals once). Before every delivery —
  first or retry — the capture's `destination_id`/`base_revision` is checked
  against what the Mac reports now (a fresh connection's `hello_ack`, or one
  `destination_query` on a reused session); a mismatch or `null` is
  `destinationChanged` and nothing is sent, so a capture never lands on a
  re-pinned or unpinned anchor without the user knowing
  (`requireCurrentDestination: false` / CLI `--destination-id` opts out).
- Bounds: one session at a time and no pending-capture queue (callers hold
  their capture and await); at most 8 in-flight requests per connection
  (`overloaded`, nothing sent beyond); inbound line bound 1 MiB (an oversized
  complete or unterminated line closes; the Mac's replies are a few hundred
  bytes); every timer is cancelled when its request completes.
- CLI `send --attempts N --retry-delay s --max-delay s --deadline s`
  (connect and submit are two bounded operations); exit codes 0 ok · 1 nothing
  stored · 2 error · **3 re-pair** · **4 retry budget exhausted** ·
  **5 destination changed on the Mac, reselect and send again** · 64/65/66
  usage/not an image/unreadable file.

`nearby-client doctor [--mac <name|fp>] [--host H --port N] [--seconds 5]
[--json]` validates one stored pairing against the running listener without
sending a capture: checks `store` (`no_pairing`, `ambiguous_pairing`,
`bad_pair_psk`), `discovery` (`not_advertised`, `unsupported_service`,
`fp_mismatch` — same Mac name, another fp: re-pair), `connect` (`unreachable`,
`handshake_refused`, `connect_timeout`), `tls` (`tls_not_1_2`,
`tls_suite_mismatch`), `hello` (the Mac's error code verbatim, e.g.
`pair_mismatch`, `too_many_sessions`; `hello_timeout`, `closed`,
`protocol_violation`) and `destination` (warn `no_destination`). One line per
check, `check <name>: ok|warn|FAIL code=<code>`, a summary with a hint, and
with `--json` the whole report as one line; exit 0 healthy (warnings allowed)
· 1 no pairing / not advertised · 3 re-pair · 4 try later · 2 other.

Tests: `swift test` in `tools/nearby-client` (47: vectors, wire shapes, TXT
validation, pair file, and a loopback-only `FakeMac` with fault injection —
drop before ack → identical re-send, idle drop, refused key terminal, remote
error terminal, destination changed/unpinned/re-pinned, deadline, cancellation,
refused port fails fast, 9th in-flight request refused, oversized inbound line
closes, request timeout, CLI exit codes 0/2/3/4/5; backpressure retried on the
same session after acks and budget-bounded, `too_many_sessions` retried after
a reconnect, image/revision/conflict refusals terminal, local image checks,
CLI hints per code; `NearbyTranscriptFixtureTests` replays duplicate delivery,
revocation — key removed by a same-port listener restart, and `pair_mismatch`
at hello — and idle-drop reconnect and compares every wire line and reconnector
event with the captured client-side transcripts in `tools/nearby-client/
Tests/Fixtures/*.jsonl`, ids/nonce/proof normalised; re-record after a
deliberate wire change with `NEARBY_CLIENT_RECORD_FIXTURES=1`; `NearbyDoctorTests`:
every doctor code and exit code against the fake Mac, Bonjour discovery,
re-salted Mac, refused port in 0.004 s). `swift test` here runs the same client against the real
stack in `NearbyReferenceClientTests` (Bonjour pair → send → identical retry →
conflict → status → forget; direct mode and listener refusals; listener
dropped after the inbox stored a capture and restarted on the same port →
duplicate delivery acknowledged, stored once; `NearbyState.forget` with a
live session → one refused reconnect, terminal, CLI exit 3; `doctor` healthy
through Bonjour, `handshake_refused` exit 3 after `forget`, `not_advertised`
exit 1 after advertising stops, nothing in the inbox;
`replaceProject`/re-pin → `destinationChanged` on reused and fresh
connections, nothing in the inbox until rebuilt; with lowered
`NearbyReceiveLimits`: `too_many_in_flight` and `inbox_full` while an
acknowledgement is held → same-session re-send acknowledged ~0.09 s after the
ack goes out, `too_many_sessions` with one session per pairing → older
connection closed during the backoff → reconnect in 0.11 s, kept open →
budget exhausted; `invalid_image` and `image_too_large` from the real
validator terminal with the session kept; same-session duplicate acknowledged
without redelivery, `revision_mismatch` and `capture_id_conflict` terminal).
Everything binds loopback only; Bonjour still resolves loopback-only services
on this machine.

Measured native behavior (separate `nearby-client` process against a served
`NearbyState`, loopback, M1 Max, macOS 26.3.1; reproduce with
`python3 tools/nearby-client/scripts/measure-native.py --out <md>`), from
[docs/evidence/nearby-client-recovery-2026-09-12T092814Z.md](../../docs/evidence/nearby-client-recovery-2026-09-12T092814Z.md) (commit d1da72c; the earlier run at T084502Z matches):
pair via Bonjour 0.76 s process-to-process; send via Bonjour 0.97 s cold, then
0.03–0.05 s; send direct 0.02 s; identical retry acknowledged again 0.02 s;
3 s listener outage with a new ephemeral port afterwards → one failed attempt,
~0.5 s backoff, re-browse finds the new port, same capture_id delivered,
3.42 s total; forgotten pairing → refused handshake, no retry, exit 3 in
0.28 s; Mac gone → 3 refused dials with backoff, exit 4 in 0.63 s (Bonjour:
2 empty browses, 2.15 s). The receive-cap codes cannot be provoked by a single
CLI process against default limits (one frame is far below the 24 MiB cap),
so they are measured in-process with lowered limits (above). In-process (real listener): drop → duplicate ack
0.21 s with a 0.2 s backoff; revoked pairing terminal 2–7 ms after the retry
begins. The serve harness (`FLASHTEX_NEARBY_SERVE_INFO=<path>`) is loopback
only unless `FLASHTEX_NEARBY_SERVE_LAN=1` is set by a human for a real iPad,
and can stage an outage (`FLASHTEX_NEARBY_SERVE_RESTART_AT`,
`_DOWN_SECONDS`) and a revocation (`FLASHTEX_NEARBY_SERVE_FORGET_AT`).

Not done / limits: pairing itself is not retried (a bootstrap code is single
use; a drop before `hello_ack` needs a new code); no Keychain; the CLI
re-checks the destination but does not offer the reselection UI transfer-v1
asks for (it exits 5 with the hint); the timing above is loopback on one
machine — Wi‑Fi to a real iPad is not measured; `FakeMac` in the package tests
is a stand-in, the real listener is only exercised from `apps/mac`.

## Launch hooks and evidence

Capture conversion: `FLASHTEX_CONVERSION_PROVIDER` (`none` / `xai`; overrides Preferences → Capture conversion), `FLASHTEX_CONVERSION_MODEL`, `FLASHTEX_AI_API_KEY` (after the Keychain; the provider's own names such as `XAI_API_KEY` still count), `FLASHTEX_KEYCHAIN_OFF=1`. The key reaches only the bridge's environment — the one child that may reach a network, by the user's choice. Live check: `FLASHTEX_CONVERSION_LIVE=1` + `FLASHTEX_CONVERSION_EVIDENCE_DIR`. See `docs/capture-conversion.md`.

`FLASHTEX_NO_ACTIVATE=1` launches without activating/focusing the window (for
automation; never steals keyboard focus). `FLASHTEX_DEBOUNCE_MS` sets the
keystroke-to-compile delay (default 0: every edit submits immediately; one
request in flight, newest buffer coalesced).
`FLASHTEX_LOG=<path>` appends timestamped worker/bridge status lines (e.g.
`status: worker exited (9)`) for automation such as `scripts/launch-check.sh`.
`FLASHTEX_TRANSCRIPT=<path>` appends every runtime-v1 JSON line the shell sends
to or receives from the worker, verbatim and in transport order, so a session
can be validated afterwards with main's `python3 scripts/check_runtime.py
<path>` (exit 0 = valid protocol; its `responses[].preview` says which replies
were `current` and which `stale_ignore`).
`FLASHTEX_AUTOATTACH=1` attaches the discovered compiler at launch and compiles
(a compiler bundled inside `FlashTeX.app` attaches by default; `=0` disables);
`FLASHTEX_SEED_FILE=<path.tex>` seeds the editor. Example (from `apps/mac`):
```sh
FLASHTEX_REPO=$(git rev-parse --show-toplevel) FLASHTEX_AUTOATTACH=1 \
  FLASHTEX_SEED_FILE=Samples/recovery-demo.tex .build/debug/FlashTeXMac
```

Screenshot of that run against crates/compiler 29221d8:
`docs/evidence/mac-shell-real-compiler-2026-09-12.png` — WORKER badge, status
`recovered`, three diagnostics with recovery notes, inline underlines. Words
crowded in that capture for two reasons, both since fixed: the compiler used
placeholder glyph widths (real Core-14 metrics arrived in de1020c) and the
preview drew with the system serif (New York) instead of Times-Roman. With
compiler de1020c the preview now matches: `docs/evidence/mac-shell-math-times-2026-09-12.png`
(inline fractions with rules, Greek, radicals, sum limits).

Rules on the legacy route (no `rules-v1` accepted): text items consisting only
of U+2500 are taken to be the compiler's fraction bars and are **approximated**
as filled rectangles (`RuleConvention`: 0.5 em per character, 0.0857 em thick,
hugging the baseline) in the preview and both PDF paths, so bars never depend on
font glyph coverage. This is a convention, not contract geometry, and cannot
establish exact rule fidelity; typed rules replace it once `rules-v1` is
negotiated (next section). Heading weight is honored only through a negotiated
`font-hints-v1` hint; without one the preview draws the regular face.

## Negotiated layout capabilities

Contract: `docs/contracts/runtime-v1-layout-capabilities.md` (additive to
runtime-v1). Every compile request carries `layout_capabilities`, by default
`["rules-v1", "font-hints-v1"]` (`ShellModel.requestedLayoutCapabilities`;
gate 3 of the contract — the consumer tests below passed first). Override with
`FLASHTEX_LAYOUT_CAPABILITIES` (comma-separated; an empty string sends no field
at all, i.e. the legacy request). The result's accepted set is bound to that
result: the banner shows `layout: legacy` or `layout: rules-v1, font-hints-v1`
for the preview currently on screen, never for the request in flight.

- Per-request binding. Each in-flight request records its capability set; a
  reply is correlated by `id` + `project_id` + `revision` *and* checked against
  that request's set, so a late reply for an older request never changes the
  renderer mode, and rapid legacy↔extended switching binds each preview to its
  own request (`ShellLayoutNegotiationTests`).
- Capability switch = its own request. Changing the set re-requests the current
  buffers under a new `id` at the **same** revision when nothing was edited
  (revisions never decrease; with auto-compile on the switch compiles by
  itself). Edits still coalesce behind the request in flight, but a switch does
  not wait: it goes out immediately, and the older request's reply — when it
  arrives later — is *superseded*: checked for violations, logged as `ignored
  stale compile_result mac-6 (...): superseded by mac-7`, and never applied.
  This is exactly `scripts/check_runtime.py`'s classification (a reply to any
  id older than the newest request is `stale_ignore`, never an error);
  `RuntimeTranscriptTests` records the shell's real exchange with the worker
  double and asserts the validator exits 0 and that its `current`/`stale_ignore`
  verdicts coincide with the ids the shell applied, including the asynchronous
  same-revision switch (slow legacy request, immediate extended request, legacy
  reply arriving last). A reply carrying an unnegotiated rule/font hint is
  rejected by both (validator exit 1, request left pending).
- Protocol violations (result rejected, banner `protocol violation: …`, log):
  an accepted capability that was not requested; a `rule` item without accepted
  `rules-v1`; a `font` hint without accepted `font-hints-v1` — including when the
  client asked but the worker did not echo acceptance (support is never guessed).
  A fixture loaded with ⌘O is checked the same way against its sibling request
  or, failing that, the set it declares itself.
- Not accepted: when the worker does not echo a requested capability the banner
  says `capability rules-v1 not accepted by the worker` (one note per missing
  capability) and the legacy rendering applies (fake worker without `%caps`,
  and any unknown capability such as `future-v9`).
- Against main's `flashtex-compiler` (main 462fb27, built from this tree): it
  **accepts both** — `RealCompilerTests.testLayoutCapabilityNegotiationAgainstRealWorkerIsExplicit`
  prints `REAL-COMPILER CAPABILITIES: requested=["rules-v1", "font-hints-v1"]
  accepted=["rules-v1", "font-hints-v1"] notes=[]` and checks end to end that
  `$\frac{1}{2}$` arrives as a typed rule (positive geometry, numerator baseline
  above `y_pt`, denominator below `y_pt + height_pt`, source navigating to
  `\frac`, no U+2500 text and no "will not survive PDF export" diagnostic),
  that every text item carries a hint (`\section` → `Times-Bold` bold, honored
  with no substitution note), that the CoreGraphics export still renders, and
  that opting out (`requestedLayoutCapabilities = []`, same revision, new id)
  returns the U+2500 legacy route with the compiler's own diagnostic.
  `testRealWorkerTranscriptPassesRuntimeValidator` runs `check_runtime.py` over
  that real exchange (four replies, all `current`, accepted sets `[both]`,
  `[rules-v1]`, `[]`, `[both]` with `future-v9` reported missing). The
  compiler does not yet style `\textbf`/`\emph` in its hints (they arrive as
  `Times-Roman` normal) — a producer limitation, reported as-is.
- Typed rules (`rules-v1`): `{"kind":"rule","x_pt","y_pt","width_pt","height_pt","source"}`
  decodes only as a typed rule — top-left corner at `(x_pt, y_pt)`, positive
  finite dimensions, magnitudes ≤ 1e6; anything else is a decode error, never
  downgraded to an unknown item. Preview and CoreGraphics export fill the
  rectangle (`RuleGeometry`; dark preview only recolors, export is opaque black
  on white); `y_pt` is never reinterpreted as a baseline. Clicking a rule
  navigates to its `source`. Once `rules-v1` is accepted the U+2500
  approximation is off for that result.
- Font hints (`font-hints-v1`): `{"family","weight":normal|bold,"style":normal|italic}`
  on text items (family 1–128 UTF-8 bytes, no control characters). Resolution
  (`PreviewFonts.resolve`): `Latin Modern*`/`lmroman*`/`Computer Modern` → the
  bundled LM masters (`apps/mac/Fonts`, registered from the repo copy in tests
  and from `Fonts/` in the app) at the requested weight/style —
  `Latin Modern Roman` bold at 12 pt is `LMRoman12-Bold`, a real CoreText face,
  with **no** substitution (`testBundledLatinModernBoldHintResolvesToLMRoman12BoldWithoutSubstitution`);
  Core-14 families `Times`/`Times New Roman`, `Helvetica`/`Arial`/`Helvetica
  Neue`, `Courier`/`Courier New` → their Core-14 faces at the hint's
  weight/style (`Times-BoldItalic`, `Helvetica-BoldOblique`, `Courier-Bold`),
  and the compiler's face-named families (`Times-Bold`, `Times-Italic`, …) are
  aliases of the family, not substitutions; anything else (e.g. `Comic Sans`)
  → Times at the requested weight/style **and** a banner note
  `font substituted: <family> [bold] [italic] → <face>`. A hint fixes style
  intent only; the shell never claims the requested metrics were preserved.
  Absent hint = legacy face selection, never a substitution.
- Unknown primitive kinds: on a negotiated route (at least one capability
  accepted) every unknown `kind` becomes an error diagnostic in the diagnostics
  list — `unsupported layout primitive 'X' at main.tex bytes a..<b on page n`
  (with "Go to source" when the item carried a `source`) — so nothing is
  silently dropped. On the legacy route unknown kinds are still skipped
  silently, as before.
- Not covered: the accessibility overlay (`FlashTeXAccessibility`, owner
  mac-accessibility) still reads only text items and the legacy U+2500 bars;
  typed rules and font hints are not yet exposed to VoiceOver. The Rust writer
  export (⌘⌥E) forwards the result envelope unchanged, including `rule` items
  and `layout_capabilities`; whether `flashtex-pdf` on main understands them is
  its owner's call and is not verified here.
- Tests: `LayoutCapabilityTests` (protocol: field round trip and limits, rule
  and font-hint validation, negotiation checks), `RuleGeometryTests`,
  `FontHintResolutionTests` (bundled LM, Core-14 aliases, substitution),
  `LayoutCapabilityPDFExportTests`, `ShellLayoutNegotiationTests` (fake worker
  `%caps` directive: negotiated compile, requested-but-not-accepted,
  substitution, unknown-kind diagnostic vs legacy skip, unrequested-shape and
  false-claim rejection, rapid switching with the in-flight reply superseded,
  same-revision switch and auto-compile switch, late/out-of-order replies,
  fixture rejection), `RuntimeTranscriptTests` (main's validator over the
  shell's transcript), and the two `RealCompilerTests` cases above (need
  `FLASHTEX_COMPILER`). The test double only *echoes* capabilities; the real
  compiler cases are the producer evidence.

## Completion and navigation

Completion (`Completion.swift`) is a pure engine over the buffer's UTF-8 bytes
with the caret in UTF-16 units, wired into the editor through a small
`NSTextView` subclass (`CompletingTextView`) whose user-completion range includes
a leading `\`. The list opens on its own while you type — `opensAutomatically`
is true for a control word (including a bare `\`) and for an argument key whose
command has completions (`\begin{`, `\end{`, `\ref{`, `\cite{`, `\label{`,
`\usepackage{`, `\input{`); a plain prose word does not open it (word
suggestions are still reachable via ⌃Space). The open fires `automaticCompletionDelay`
(50 ms) after the keystroke, so one fast burst of typing costs one document
scan rather than one per character; Esc or ⌃Space also open it explicitly at
any time. It is the editor's own non-activating completion
list (`CompletionPopup`, a child panel that never becomes key): ↑/↓ or Tab/⇧Tab
choose (wrapping, each choice announced to VoiceOver as "n of m: candidate, kind,
origin"), Return/Enter inserts the chosen entry over the partial token as one
undo step, Esc closes; typing narrows the list and any other caret move closes
it. Candidates are computed off the main thread and delivered only while the
buffer and caret are unchanged (`CompletionLatencyTests` measures pickup,
narrowing, arrow and Return latency best-of-N). Sources, in rank order, at most
12 entries:

1. `\end{X}` for every `\begin{X}` before the caret that is still unclosed
   (detail names the byte of the `\begin`).
2. Commands the compiler supports, generated from the compiler's own
   machine-readable inventory `crates/compiler/supported/supported-latex.json`
   (schema `flashtex-supported-latex/1`, regenerated by `flashtex-compiler
   --supported json` and drift-gated in the compiler crate). The file is
   bundled as the SwiftPM resource `Sources/FlashTeXMac/Resources/
   supported-latex.json`, a byte-identical copy kept by
   `scripts/sync-supported-latex.sh` (`--check` in CI and `make-app.sh`, which
   also copies it into `Contents/Resources`; `CompletionTests.
   testBundledInventoryMatchesTheCompiler` compares the two by sha256).
   `Completion.Vocabulary` decodes it at first use: only `renders: true`
   commands; text-mode entries (`text_dispatch`, `text_style`, `text_size`) in
   file order, then `\\`, then math structures, operator names and symbols
   (the `glyph` is the detail, "math · symbol α"); a command the compiler
   accepts in both modes (`\textbf`, `\quad`) is one text entry whose detail
   also states the math behaviour. Argument shapes (`\frac{num}{den}`,
   `\sqrt[index]{x}`) come from the inventory and become the snippets;
   `Vocabulary.argumentOverrides`/`snippetOverrides` are the only hand-written
   parts (`\left( \right)`). Environments come from the inventory's
   `environments`. Hover documentation (`EditorIntelligence.CommandDocs`) may
   only name inventory commands or its explicit `beyondCompiler` list
   (`testCommandDocsNameOnlyKnownCommands`). Pass another `supported:` list to
   restrict the set.
3. Commands typed elsewhere in the document that are not in that list, marked
   "not supported by this compiler version" (plus the compile result's
   diagnostic message when one names the command).
4. After `\begin{`/`\end{`: environment names (open ones first, then `document`,
   then names seen in the buffer); after `\ref{`/`\eqref{`/`\pageref{`/`\autoref{`:
   `\label` arguments seen in the buffer plus the project index's labels; after
   `\cite{` (and the other citation commands, also after a comma in the key
   list): `\bibitem` keys and the project index's citation keys; after
   `\usepackage{` (also after a comma): common package names; after
   `\input{`/`\include{`/`\includegraphics{`: the project's document paths
   (`SourceEditorView.projectFiles`, the model's `documents`), matched on the
   path or its basename; after `\label{`: a key whose prefix follows the
   innermost open environment (`fig:` in `figure`, `tab:` in `table`, `eq:` in
   a math display, `thm:`, `lst:`, else `sec:`) plus the enclosing heading's
   slug, made unique against the document and the project index, and the bare
   prefix alone. Argument keys are scanned as one token including `:`, `-`,
   `_`, `.`, `/`, `+`, `*` and digits (`\ref{eq:ma` completes `eq:main`).
5. Prose: words longer than 3 characters from the document, frequency-ranked,
   ASCII case-insensitive prefix, triggered after 2+ letters. The word being
   typed is not counted as its own completion.

Matching is exact, then prefix (each in table/source order); when nothing
starts with the typed characters, subsequence matches are offered instead
(`\sbs` → `\subsection`, `\ref{main}` → `eq:main`), never mixed under prefix
rows (`Completion.matchRank`/`fuzzyFilter`).

**Snippets and tab stops** (`Completion.Snippet.stops`, `SnippetTests`): a
command with braced arguments inserts its shape with the caret in the first
braces and a Tab stop at every later one and at the end (`\frac{|}{}` → Tab →
`\frac{ab}{|}` → Tab → after the snippet); `\left` inserts `\left( \right)`.
`\begin{…}` inserts the environment's template with the current line's
indentation: `itemize`/`enumerate` start with `\item `, `description` with
`\item[] `, `figure` with `\centering`, `\includegraphics[width=0.8\linewidth]{}`,
`\caption{}` and `\label{fig:}` (three stops), `table` with `\centering`, a
`tabular`, `\caption{}` and `\label{tab:}`; every other environment an empty
indented middle line. While the snippet is active Tab / ⇧Tab move between the
placeholders (typing at a placeholder keeps it at the start of what was typed;
an edit across a placeholder ends the snippet), Esc or moving the caret out of
the snippet leaves it, and Tab is a Tab again.

**Signature help** (`SignatureHelp.swift`, `SignatureHelpTests`): typing `{` or
`[` right after a command name — or ⌘⇧Space anywhere inside a command's
argument — shows a small panel above the caret with the command's argument
pattern (`\frac{num}{den}`, the argument the caret is in emphasised, optional
`[…]` groups counted) and its one-line documentation (`CommandDocs`, falling
back to the vocabulary description). It follows the caret between arguments
and closes on `}`, Esc, or when the caret leaves the argument (or the line;
the scan is bounded to the caret's line and skips commented text). The
inserted `\frac{|}{}` snippet opens it too. Off when the completion-list
preference is off.

**Editor niceties** (`SourceEditorView.swift`, `EditorIntelligence.swift`,
`EditorKeyHandling.swift`): `{`, `[` and `$` are auto-closed and the closer
typed over (Backspace between an empty pair removes both); `\(` and `\[`
auto-close with `\)` / `\]`, both halves typed over (`ShellModel.autoClosePairs`,
default `{ [ $ (`; the Preferences "auto-close brackets & math" switch gates
all of them). A completion snippet's own placeholder closer (`\section{}`'s
`}`) is tracked the same way, so typing over it overtypes instead of doubling
it. Tab indents: a multi-line selection gets the indent unit (spaces × width,
or a tab, per the Preferences "Indent with" setting) prefixed to every line
it touches, and a caret or single-line selection just inserts it; Shift-Tab
always outdents the touched line(s) by up to one unit, selection or not.
While a completion list or an inserted snippet's placeholders are active, Tab
still means those instead (below). Return keeps the line's indentation,
indents once more after `\begin{env}` and adds the matching `\end{env}`;
Return at the end of a `\item …` line continues the list with a new `\item `
(`\item[] ` for a description entry; a bare `\item` line just breaks). ⌘/
toggles `% ` on every line the selection touches (all commented → uncomment,
`%` with or without a space; otherwise comment the non-blank lines; one undo
step "Toggle Comment"). The delimiter pair around the caret is highlighted
(`BraceMatcher`).

Commands trigger on `\` (empty prefix lists everything supported). Invalid
carets (negative, past the end, inside a surrogate pair) and malformed input
(`\begin{`, stray braces, runs of backslashes) yield no suggestions and never
trap. Measured on a 1 000 069-byte buffer (`CompletionTests`, M1 Max, 2026-09-12):
release build words 1.9 ms, commands 0.8 ms per call (XCTest `measure` of one
word + one command completion: 2.6 ms average, RSD 0.9%); debug build 8.4 ms /
5.3 ms (`measure` 13.7 ms). Scans jump between candidate bytes with `memchr`
and decode only matches, so the cost is proportional to candidates, not bytes.

Navigation (`Navigation.swift`, `Navigate` menu):

- **Go to Matching** (⌘⇧D): from `\ref{X}`-style commands to `\label{X}`; from
  `\label{X}` to its references in turn (wrapping); `\begin{X}` ↔ `\end{X}` with
  same-name nesting. Pure functions over the current buffer with byte-exact
  ranges converted to UTF-16 for the selection; misses are explained in the footer.
- **Next / Previous Diagnostic** (⌘⇧] / ⌘⇧[): cycles (wrapping) through the
  result's diagnostics that have a source in the active document, ordered by
  their position in the current buffer. Each jump goes through
  `ShellModel.navigate(to:expectedText:)`, so a diagnostic whose span overlaps an
  edit made since the compile is refused with "recompile to navigate" rather
  than selected on the wrong text; the others stay reachable. Diagnostics with
  null `source` are skipped and counted in the footer note.
- **Reveal Caret in Preview** (⌘⇧J): selects the full source span of the preview
  item under the caret (`CaretSync`) so the preview highlight and page scroll
  follow, and names the page and item. It is also the manual override for
  automatic following (`CaretFollow.swift`): it scrolls at once — no debounce,
  and regardless of the "Preview follows the caret" preference, on or off —
  and re-arms following (once the preference is on) after a manual preview
  scroll had stopped it. Automatic following itself re-arms the same way on a
  text edit, or on its own the moment the caret lands on a different source
  line or a different preview page (a caret move that stays within the same
  line does not re-arm it).

Without a compile result the navigation commands are disabled and, if invoked,
explain that nothing is loaded.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| ⌘, | Settings window (editor preferences: font, wrapping, tab width, indent, appearance, auto-close brackets & math, completion list; Tab walks the controls top to bottom) |
| ⌘O | Open LaTeX file… (becomes the `main.tex` entry document; compiles if a worker is attached) |
| ⌘⌥N | New Project… sheet (folder, name, template: Blank article / Article with sections / Report with chapters / Homework sheet; writes `<folder>/<name>/main.tex` plus its `\input`/`\include` members, opens `main.tex` as the entry document with the include tree in the sidebar; asks before replacing existing template files) |
| ⌘N | New File… sheet (also the sidebar's + button and the project row's context menu): a rooted `.tex` name, subfolders allowed, never above the project root; "Insert `\input` at the caret" (on by default while the entry document is active) is one undoable edit; the file opens in a tab |
| ⌘S / ⌘⇧S | Save / Save As… (UTF-8; header shows "— edited" when dirty) |
| Edit > Restore Discarded Buffer | Brings back the unsaved text replaced by a "Discard" decision when opening another file |
| ⌘⇧O | Open compile result fixture… (sibling `-request.json` seeds the editor) |
| File > Reload Fixture | Reload Fixture (developer-only, no shortcut — confirms before replacing a real/unsaved document) |
| ⌘⇧K | Attach built compiler (`$FLASHTEX_COMPILER` or `crates/compiler/target/…`) |
| File > Export PDF (exact, v2)… | Exact route: the loaded v2 display list through `flashtex-pdf-exact from-v2` (`$FLASHTEX_PDF_EXACT`, bundle, or `crates/pdf/target/…`): original GIDs, embedded font programs, typed rules; refusals name the item |
| ⌘⇧R | Attach render pipeline (`$FLASHTEX_RENDER`, the app bundle, or `crates/render-pipeline/target/…`): the Latin Modern-metric producer, so the preview shows Computer Modern-style text |
| ⌘K | Attach worker executable… |
| ⌘B | Compile now (auto-compile also runs 250 ms after edits) |
| ⌘⇧E | Export PDF… (CoreGraphics, always white) |
| ⌘⌥E | Export PDF via Rust writer… (`flashtex-pdf --verify`, always white) |
| ⌘P | Print… (compiled document PDF, same CoreGraphics bytes as Export PDF…; system print panel; page size follows the PDF) |
| File > Print Source… | Print Source… (editor text with line numbers, monospaced, from a copy so the live editor is untouched) |
| ⌘⌥P | Pin insertion point at caret (capture destination anchor) |
| Edit > Open Capture Proposal… | Open capture proposal… file (review sheet; ⏎ approves, inserts one undoable edit; no shortcut since ⌘⇧I moved to the Captures inspector) |
| ⌘⇧I | Toggle Captures inspector (View; also the toolbar's Captures button): captures from the paired iPad with image, instruction and state (received → converting → proposal ready → inserted), the proposed LaTeX/TikZ, Insert at caret / Edit / Review… / Reject; opening it starts advertising and attaches the bridge; Pairing code… is one click |
| ⌘⇧U | Submit sample capture… (PNG/JPEG → `capture_submit` through the attached bridge) |
| ⌘⇧G | Convert capture (`capture_convert` for the latest received capture) |
| ⌘⇧N | Nearby Companion… (advertise, pairing code, paired devices, received captures; Return shows or resumes a pairing code, Esc cancels it or dismisses a banner, Tab walks Advertise → pairing controls → Forget → Clear; the step indicator, status row and every transition are VoiceOver text) |
| Edit > Rename Citation… | Rename citation window (reviewed `plan_citation_rename` across the project → one `apply_group`) |
| ⌘⇧P | Command palette (View; also the toolbar's Commands button): every command in this table with its menu and shortcut; type to filter, ↑/↓ choose, Return runs, Esc closes |
| ⌘= | Zoom in preview (View; also the preview header's + button or a pinch): multiply the fit-width zoom by 1.25, up to 4x; wide pages scroll horizontally |
| ⌘- | Zoom out preview (View; also the header's − button): divide by 1.25, down to 0.25x fit width |
| ⌘0 | Actual size preview (View): 1 PDF point per screen point when the 0.25x…4x zoom bounds permit it |
| ⌘9 | Fit width preview (View): reset zoom to 1x so the widest page fits the pane; double-click the header percentage does the same |
| ⌘⇧9 | Fit page preview (View): zooms so the tallest page's full height fits the pane (within the 0.25x…4x bounds); also in the preview header on hover |
| ⌘⌥= | Increase editor font size (View; also a pinch over the editor): +1 pt up to 36 pt, persisted as the Settings font-size preference; gutter and highlighting follow |
| ⌘⌥- | Decrease editor font size (View): -1 pt down to 8 pt |
| ⌘⌥0 | Reset editor font size (View): back to the default 13 pt |
| ⌘⇧M | Toggle Problems panel (View): the grouped diagnostics list under the editor and preview with a severity filter, jump, explanations and Fix…; the sidebar's Problems rows and the status bar counts open it too |
| ⌃⌘V | Toggle Vim keybindings (View; also the Settings switch, default off): modal editing in the source editor — a Vim status line at the bottom of the editor pane shows `-- NORMAL --` / `-- INSERT --` / `-- VISUAL --`; see *Vim keybindings* below |
| Edit > Durable History… | Durable History window (undo/redo on the helper's edit ledger: Refresh, Undo, Redo, Retry/Discard after an uncertain reply, retention gauge, both stacks) |
| ⌘F | Find… (opens the source editor's find bar; AppKit's built-in incremental search) |
| ⌘⌥F | Find and Replace… (opens the find bar already showing its Replace row; a replacement is one undoable edit, so ⌘Z undoes it and the preview recompiles) |
| Edit > Find Next | Find Next (selects the next find-bar match; no key equivalent — ⌘G is Find in Project's Next match, ⇧⌘G is Convert Capture — Return in the find bar's search field does the same) |
| Edit > Find Previous | Find Previous (selects the previous find-bar match; no key equivalent for the same reason — Shift-Return in the find bar's search field does the same) |
| ⌘E | Use Selection for Find (sets the focused editor's selection as the find bar's search string) |
| ⌘J | Jump to Selection (scrolls the focused editor's current selection into view and centers it) |
| ⌘⇧F | Find in Project… window (case-sensitive literal search of the durable project source; Return searches or goes to the selected match, ↑/↓ move the selection, Esc closes; Plan Replacement / Apply for reviewed replacement) |
| ⌘G | Next match (while the Find in Project window is key: selects the next match, wrapping, and goes there) |
| ⌘Z | Undo (including an approved capture insertion) |
| Esc / ⌃Space | Completion popup (supported commands, `\end{…}` for open environments, labels, citation keys, document words; never takes the keyboard from the editor) |
| ↑ / ↓ / Tab / ⇧Tab / Return | Completion list keys, while the list is open: ↑/↓ or Tab/⇧Tab choose the candidate (wrapping; VoiceOver announces “n of m: candidate, kind, origin”), Return/Enter inserts it over the typed token, Esc closes without inserting; typing narrows the list, any other caret move closes it |
| ⌘⇧Space | Signature help for the command whose argument the caret is in (also opens on `{`/`[` typed after a command name; `}`, Esc or leaving the argument closes it) |
| ⌥⇧↓ / ⌥⇧↑ | Duplicate the caret's line — or every line the selection touches — below / above itself, caret on the copy so the key repeats |
| ⌘/ | Toggle `% ` line comment on the selection's lines |
| ⌃I | Re-indent Lines (selected lines, or the caret's line; LaTeX-aware; one undo step). Not Tab; Vim does not bind ⌃I; ⌘⇧I is Toggle Captures |
| Edit > Re-indent Document | Re-indent Document (same rules over the whole buffer; one undo step; no shortcut) |
| ⌘⌥← | Fold the innermost environment or section at the caret (first line stays visible with an inline …; hidden characters stay in the buffer) |
| ⌘⌥→ | Unfold the innermost folded region at the caret |
| ⌘⌥⇧← | Fold All environments and sectioning blocks |
| ⌘⌥⇧→ | Unfold All folded regions |
| ⌘⇧D | Go to matching `\begin`/`\end` or `\label`/`\ref` |
| ⌃⌘J | Go to definition of the command/environment under the caret (`\newcommand`, `\def`, `\DeclareMathOperator`, `\newenvironment`; ⌘-click does the same, hover peeks the body) |
| ⌘⇧T | Go to symbol: fuzzy picker over every heading, environment and label of the open documents |
| ⌘⇧A | Select environment: the innermost `\begin{X}`…`\end{X}` around the caret, again for the enclosing one (a caret on `\begin`/`\end` highlights its partner) |
| ⌘⇧W | Wrap selection in environment… (whole lines as an indented block, otherwise inline; one undoable edit) |
| ⌥⇧R | Rename symbol: the `\label` key or user command under the caret across the open documents (Plan → Apply; one undoable edit per document, one guarded `apply_group` per file with the helper) |
| ⌘⇧] / ⌘⇧[ | Next / previous diagnostic (refused if its span was edited since the compile) |
| ⌘⌥] / ⌘⌥[ | Next / previous occurrence within the diagnostics panel's selected group (wrapping; the row reads "k of n") |
| ⌘⌥C | Copy diagnostics as text (`path:line: error/warning: message` lines for the selected row, all when none; ⌘C while the list has the keyboard) |
| ⌘⇧J | Reveal caret in preview (selects the item's source span) |
| Click preview text | Select its source (UTF-8 span → UTF-16; refused if edited since compile) |
| Help > FlashTeX Accessibility Help | Help window: focus order, what VoiceOver reads in each pane, every command above |

The compiler rejects request lines over 8 MiB with an `error` envelope, which the
banner shows; the shell rejects response lines over 16 MiB.

## Editor keys

A few physical keys mean different things depending on editor mode, so no
single row in the table above could own them (each shortcut cell there names
exactly one command). They are not menu items or command-palette entries —
only the meaning that applies once the higher-priority modes below are
inactive (completion list, then snippet placeholders) reaches the plain
editor behavior.

| Keys | Behavior |
|---|---|
| Tab / ⇧Tab / Esc | While an inserted snippet is active (no completion list open): next / previous placeholder (`\frac{|}{}`, environment templates), Esc leaves the snippet |
| Tab / ⇧Tab | Otherwise (no list, no active snippet): Tab indents (a multi-line selection: every touched line; a caret or single-line selection: inserts the indent unit at it); ⇧Tab always outdents the touched line(s) by up to one unit |
| Return | Auto-indent; after `\begin{env}` indent and add `\end{env}`; at the end of a `\item …` line continue the list |

## Vim keybindings

`VimMode.swift` (`VimModeTests`): a modal state machine layered on the editor
text view's `keyDown`, active only while the "Vim keybindings" preference is on
(Settings ▸ Typing, or View ▸ Toggle Vim Keybindings ⌃⌘V; default off — with it
off nothing is intercepted and `CompletionLatencyTests` measures the plain
editor). The mode and the `:`/`/` line being typed show in a status line at
the **bottom of the editor pane** (`VimStatusLine`, ContentView.swift), where
vim puts the status line of the window being edited — not in the window's
status bar, which the Problems panel would push two panes below the caret.
With the preference off the line takes no space at all.

- **Modes**: normal, insert (`i a I A o O s S c C`), visual `v`, visual line
  `V`, replace-one `r{char}`; Esc or ⌃[ returns to normal. A mouse selection
  in normal mode enters visual mode.
- **Insert mode is the plain editor**: only Esc / ⌃[ are intercepted, and never
  while marked text (an input-method composition) exists — IME, dead keys,
  completion, snippets and signature help behave exactly as without Vim. Esc
  closes the completion list *and* leaves insert. ⌘-shortcuts are never
  intercepted in any mode.
- **Counts** on motions, operators and commands (`3dw`, `2d2w`, `5x`, `2G`).
- **Motions**: `h j k l w b e W B E 0 ^ $ gg G { } ( ) f F t T ; , % H M L`,
  ⌃D ⌃U ⌃F ⌃B. `%` matches `( ) [ ] { }` and, on a `\begin`/`\end`, its partner
  (`EditorNavigation.environmentPair`).
- **Operators** `d c y > <` with motions, doubled for lines (`dd cc yy >> <<`),
  and **text objects** `iw aw i( a( i[ a[ i{ a{ i" a" i$ a$` (inline math) and
  `ie ae` (the innermost `\begin{X}…\end{X}`, `EditorNavigation.enclosingEnvironment`;
  `ie` is the body — whole lines when the delimiters sit on their own lines).
- **Commands**: `x X s S D C Y p P J gJ u ⌃R . ~`, marks `m{a-z}` / `'{a}` /
  `` `{a} ``, `zz`/`zt`.
- **Registers**: unnamed (default), named `"a`–`"z`, and `"+` / `"*` for the
  system clipboard (`"+yy`, `"*p`).
- **Search**: `/` and `?` with incremental preview (Esc restores the caret),
  `n N *`; smart-case; the term is put on the find pasteboard so ⌘G / Edit ▸
  Find ▸ Find Next continue it in the find bar.
- **`:` commands**: `:w` (save), `:q` / `:q!` (close the active non-entry
  document; the entry document cannot be closed), `:wq` / `:x`, `:e path`
  (open a project document), `:[%|N,M|'<,'>]s/a/b/[g]` (literal text, one undo
  step, through the editor's edit path so the model recompiles), `:noh`,
  `:set nu` / `:set nonu` (line-number gutter).
- **Undo**: `u` / ⌃R are the editor's undo manager. Coalescing is broken at
  insert-session boundaries, so one insert session is one step; every operator
  is one step.

Not yet: `.` repeats an operator + motion/text object and re-inserts the
text of an insert session, but not visual-mode changes; `o` + typed text and
`cw` + typed text are two undo steps (open/delete, then the insert); no `gu`
`gU` `gq` `=`, no `iS aS ip ap it at` objects, no regular expressions in
`/` and `:s` (literal, smart-case), no `:g`, `:'a`, `q` macros, jump list
(⌃O/⌃I), `Ctrl-V` block mode, `R` replace mode, or `.vimrc` mappings; marks
do not follow edits; `H`/`M`/`L` use the visible rect without `scrolloff`.

## Targets

- `FlashTeXProtocol` — Codable models for runtime v1 (`RuntimeV1`) and the
  capture bridge (`TransferV1`), byte-offset conversion, changed-region diffing.
- `FlashTeXMac` — the app. `BridgeClient` (JSON Lines transport, id-correlated
  replies, 12 MiB line limit), `BridgeSession` (bridge-side document shadow,
  destination, captures, edit ledger, reconciliation), `ShellModel+Bridge`.
- `FlashTeXMac` also holds `LineProcessClient` (shared bounded JSON Lines
  process transport) and `EditLedgerClient` (edit-ledger helper protocol).
- Tests (136 across all targets, of which `RealCompilerTests`, `RustPDFExportTests`,
  `RealBridgeTests` and `RealEditLedgerTests` are gated on `FLASHTEX_COMPILER`,
  `FLASHTEX_PDF`, `FLASHTEX_BRIDGE` and `FLASHTEX_EDIT_LEDGER`): completion
  (prefix/trigger rules, unclosed `\end{}`, unsupported marks, non-ASCII and
  invalid carets, 1 MB latency) and navigation (label/ref incl. Unicode, nested
  begin/end, diagnostic cycling/wrap/refusal on the multipage sample, caret
  reveal); bridge transport round trip of every transfer-v1 request type incl. error envelopes,
  garbage/oversized/trailing lines and oversized requests; edit-ledger helper
  round trip (initialize/apply/dedup/undo/confirm/error codes); shell ↔ fake
  bridge + fake ledger flow (open → edit → pin → submit → received → convert →
  review → prepare → verify → durable apply → adopt once → `capture_applied` →
  confirm, duplicate approval no-op incl. after undo, edited-LaTeX refusal,
  stale edit refused by shell and helper, provider error as text, reject
  forwarded, no-helper fail-closed); fault/recovery (persistence failure inserts
  nothing and sends no receipt, file-backed export before receipt, export
  failure withholds the receipt while the durable commit stands, corrupt or
  unreadable store fails closed, transient status failures retain
  transactions and snapshots, missing receipt replayed from the retained
  snapshot only for an exactly matching prepared edit, stalled bridge never
  blocks the main thread + backpressure, detach/reattach ignores old-session
  events, edits during reconciliation); real bridge (durable receipt,
  duplicate/conflict, `provider_disabled`, `proposal_missing`, reject,
  `capture_missing`); real edit ledger (durable apply with on-disk hash check,
  duplicate edit ID refused, recovery export/import incl. stale token, undo
  keeps dedup, session metadata, second writer refused, full shell flow);
  nearby listener (TLS-PSK round trip of `hello` /
  fixture `capture_submit` / `destination_query` through an `NWConnection`
  client with the same PSK, wrong key and unknown identity refused before any
  line is parsed, oversized complete and unterminated lines close, message
  before `hello`, cross-pairing `hello` with the wrong proof, nonce reuse,
  bootstrap → long-term PSK hand-over with the old key refused after a
  same-port restart while the live session survives, Bonjour advertising
  reaches `.ready`, pure session validation), HKDF/proof vectors, pair store
  round trip with 0600 and corrupt-file handling, `ShellModel` inbox and
  destination, `NearbyState` pair → capture → forget → same-port restart
  flow, plaintext peer refused without parsing while TLS peers keep working,
  nearby capture forwarded through the fake bridge (durable ack, error
  pass-through, inbox fallback after detach); plus the earlier: oversized complete line, trailing bytes at EOF, unsolicited/mismatched result correlation; inline diagnostic marks (byte→UTF-16, rebase/drop, path filter,
  sample slice, temporary-attribute-only); Rust-writer export (gated on
  `FLASHTEX_PDF`), missing-binary error; source mapping (shift/refuse/multi-byte/expected-text), stale
  navigation refusal and rebase, auto-compile debounce/coalescing, latency; PDF export (fixture → 612×792 page containing the item text,
  two-page synthetic sizes, unknown-kind skipping, page-less result); anchor/rebase/reselection logic, review flow with duplicate
  suppression, capture fixture decoding; caret sync (multi-page sample slices
  byte-exactly, `itemsContaining` boundaries incl. inside a multi-byte scalar,
  `ShellModel.caretByte`/`caretItems`, sibling request discovery); end-to-end
  against the real compiler (gated on `FLASHTEX_COMPILER`); plus fixture
  decoding, version/type rejection, unknown kinds, UTF-8→UTF-16
  conversion with multi-byte scalars, `ShellModel` load/navigate/stale behavior,
  line splitting/encoding, and a round trip through `Tests/.../fake_worker.py`
  (a Python test double, not a compiler) including error/garbage/exit paths and
  the stale-revision guard.

## v2 preview (experimental)

An opt-in consumer for the EXPERIMENTAL rendering-v2 display list
(`docs/contracts/rendering-v2-proposal.md`, `protocol/rendering-v2.schema.json`;
not a negotiated production wire). The runtime-v1 preview above stays the default;
this pane only exists to prove the consumer gates on real pipeline output. It does
not replace, negotiate, or change the v1 path.

- Input, live: while the pane is visible the shell adds `display-list-v2` to the
  compile request's `layout_capabilities` (`docs/contracts/runtime-v1-display-list-v2.md`;
  `ShellModel.setLiveV2`). A producer that accepts it (flashtex-render ≥ 4888a67) echoes
  it and writes the `display_list` envelope as one sibling line right after the
  `compile_result`; `WorkerClient.decode` routes `protocol_version: 2` + `display_list`
  lines (header probed by a byte scan, `RenderingV2Fast.header`) to
  `ShellModel.receiveDisplayListV2`, which applies a line only when its `id` is the applied
  result's id and that result accepted the capability — anything else is stale or
  unsolicited and is dropped (`V2Live` counters; unsolicited → protocol-violation status).
  The header shows LIVE / "v1 only" and "frame revision N — applied result is M" when a
  result came without a frame (failed or declined per request). The v1 pages of the same
  result paint first and stay the product preview. Old producers ignore the capability;
  the pane never requests it while hidden. The preview-controller (helper) route does not
  forward the line yet.
- Images (`display-list-v2-images`, `protocol/proposals/display-list-v2-image.md`,
  consumer co-signed): `setLiveV2` requests `display-list-v2-images` alongside
  `display-list-v2` and the compile request carries `project_root` (the open project's
  directory) so the producer can size `\includegraphics{…}` files (png/jpeg/pdf, inside
  `figure`/`table` floats). Image items (`kind: "image"`: tick box, unit-square
  `transform`, `image{image_id = sha256, byte_length, format, path, pixel dims | pdf_page/
  pdf_box/pdf_rotate}`) are decoded by both readers and validated like rules
  (`RenderingV2.validateImageResource`; unknown fields tolerated). Bytes are NOT on the
  line: `V2ImageStore` (V2ImageStore.swift) reads `path` under the project root with the
  same rooted, symlink-refusing rule as the project-files helper
  (`ProjectDocuments.rootedFile`; the helper's `read` is text-only, so binary assets take
  the rooted local read), verifies `byte_length` and SHA-256 before decoding, and caches
  by `image_id`. A mismatch refuses that item only: the frame is kept, the item paints
  nothing, and the pane header shows one non-modal notice per path ("stale image:
  figures/plot.png"; "image unavailable: …: a.png is a symbolic link" / "no project root").
  PNG/JPEG paint through `CGImage`, PDF pages through `CGPDFDocument` with the producer's
  `pdf_box`/`pdf_rotate` mapped onto the unit square (`V2PreparedImage.pageToUnit`), all
  clipped to the tick box and placed by the item's transform (`[a, −b, c, −d, e, H − f]`
  in PDF space), in the preview bitmap and in the CoreGraphics `Export PDF (v2)…` alike.
  A click on the box navigates to the `\includegraphics` command; selection ignores
  images. `V2PageCache` keys on the image root too, so a page prepared under another root
  (or with no root) is never reused. Gap: `flashtex-pdf-exact from-v2` (crates/pdf,
  Commander-owned) still refuses image items ("glyph_run and rule only"), so the exact
  export of a frame with images fails naming the item; use `Export PDF (v2)…` for those.
  Tests: `V2ImageTests` (generated PNG/JPEG/PDF fixtures, stale-hash and symlink
  refusals, rotated PDF box, cache keying, request wiring, and a `FLASHTEX_RENDER`-gated
  round trip through the real producer with `project_root`).
- TikZ paths (proposal `path-v0`, `crates/render-pipeline/src/display.rs` `PathItem`;
  producer: Kabir's tikz-min work). Whenever `display-list-v2` is negotiated — there is
  no separate capability to request, `protocol.rs` gates only `display-list-v2` and
  `display-list-v2-images` — a `tikzpicture` arrives as `kind: "path_fill"`
  (`fill_rule` nonzero|evenodd) and `kind: "path_stroke"` (`stroke{width, cap, join,
  miter_limit, dash{array, phase}?}`) items with `path` commands `["m",x,y]`, `["l",x,y]`,
  `["c",x1,y1,x2,y2,x,y]`, `["z"]` in page ticks (top-left, y down; no transform on the
  wire), optional `clips` (`kind: "path"` + `fill_rule` + `path`), `paint` and the usual
  provenance (the producer attributes every path to the whole `tikzpicture` span), and
  `required_features` gains `path_fill` / `path_stroke` / `clip` (all in
  `RenderingV2.knownFeatures`). Both readers decode them
  (`RenderingV2.Path`/`PathCommand`/`Stroke`/`ClipPath`; `RenderingV2Fast` typed path);
  validation is fail-closed: 1…65536 commands per path or clip, ≤16 clips, exact ticks, a
  move before any line/curve/close, positive stroke width, finite miter limit ≥ 1, a
  1…32-entry nonnegative dash with a positive entry, paint in 0…1, provenance, and used
  features declared. `V2PathGeometry` (V2PathGeometry.swift) turns commands into a
  `CGPath` in PDF space (y flipped, 7-significant-digit quantized like every other
  coordinate) and `V2PreparedPath.draw` paints it inside the one `GlyphRunRenderer.draw`
  routine — fill rule, width/cap/join/miter/dash in points from ticks, every clip applied
  in a saved graphics state — so the preview bitmap and the CoreGraphics `Export PDF
  (v2)…` paint identical geometry; the dark preview inverts path ink like rules and text.
  Hit-testing is exact ink containment (`CGPath.contains` by fill rule, or the stroked
  outline at least 2 pt wide, and inside every clip) and a click navigates to the
  picture's source span; `DisplayListDelta.pageDigest` covers commands, op, clips, paint
  and provenance and relocation moves path sources; `V2PageCache` keys on page bytes so
  paths are covered by construction. Gap: `flashtex-pdf-exact from-v2` (crates/pdf
  `v2.rs`, Commander-owned) refuses `path_fill`/`path_stroke` ("glyph_run, rule and image
  only"), so the exact export of a frame with a `tikzpicture` fails naming the item; use
  `Export PDF (v2)…` for those. Tests: `V2PathTests` (both readers agree and round-trip,
  malformed shapes refused, a triangle / Bézier circle / dashed line with arrowhead /
  clipped fill painted offscreen with pixel assertions in preview and export, hit tests,
  dark inversion, digest and relocation, and a `FLASHTEX_RENDER`-gated live compile of
  `\draw (0,0) -- (2,1); \fill[red] (1,1) circle (0.3);` asserting path items arrive,
  decode, prepare and paint red at the disc centre).
- Input, file: a `display_list` JSON envelope written by `flashtex-render --v2 out.json`.
  Open it with `File > Open Display List (v2)…`, or launch with `FLASHTEX_V2_FILE=<json>`
  (`FLASHTEX_PREVIEW_V2=1` starts with the switch on). The preview header's
  "v2 pane" switch flips between the v1 and v2 panes.
- Keeping up with typing (`docs/evidence/mac-preview-v2-live-2026-09-12.md`): one
  preparation in flight, the newest arrival waits and lists in between are dropped
  undecoded (coalescing; strict supersession alone starved visible progress: only the last
  frame painted); `RenderingV2Fast`, a typed byte-level reader for the envelope (9 ms for
  the 1.9 MB two-page demo envelope against 75 ms with JSONDecoder, same values, JSONDecoder
  remains the arbiter of what is rejected); a new frame is pre-rasterized off-main at the
  pane's last pixels-per-point and its bitmaps are installed right before it is published,
  so the pass that shows it blits at once; the header is its own view, bitmaps are observed
  per page and page views are Equatable, so a bitmap or caret change redraws one page.
  Measured with flashtex-render 4888a67 on demo.tex: keystroke→paint p50 89 ms (p95 116)
  through the v2 pane, 40 ms through the v1 pane with the same producer; every keystroke
  painted, every frame published.
- Fail closed (`FlashTeXProtocol/RenderingV2.swift`): unknown `protocol_version`,
  message `type`, item `kind`, `required_features`, font `format`, or an undeclared
  font/document reference, a glyph ID outside `1..<glyph_count`, a cluster that does
  not partition the run text on UTF-8 boundaries, malformed carets/rects/paint, or
  non-integer geometry is a diagnostic-bearing `ValidationError`. The validator also
  applies crates/rendering-core's `DisplayList::validate` structural rules: every
  feature the list uses must be declared in `required_features` (`glyph_run`, `rule`,
  and always `rgba-srgb`/`cluster-actualtext`; `static-truetype` is deliberately not
  derived from glyph runs because the pipeline paints Latin Modern as `opentype-cff`),
  every cluster is referenced by at least one glyph, source ranges lie within the
  declared document `byte_length`, every tick and tick sum stays within ±(2^53−1),
  document paths follow rendering-core's `path` rule, and collection sizes are
  bounded (`RenderingV2.Bounds`: documents 1…4096, fonts ≤256, pages ≤10000, items
  ≤100000, glyphs/clusters 1…65536, hit rects 1…128, carets ≤128). The pane then
  shows the code and message and NO page: a refused list never renders partially.
- Fonts by content hash only (`GlyphRunRenderer.swift`, `V2FontStore`): every
  `.otf`/`.ttf` in the existing `PreviewFonts.latinModernSearchPaths` directories
  (bundle `Fonts`, `apps/mac/Fonts`, `FLASHTEX_LM_DIR`) is SHA-256'd once; a
  manifest entry resolves only if its `sha256` matches a file exactly, and the file's
  byte length, glyph count, units per em and PostScript name must agree with the
  manifest. A run whose font is not bundled refuses the whole frame with
  `font_resource_unavailable: font resource <sha256> (<name>) unavailable`. Platform
  font names are never an identity; nothing is substituted.
- Preparation, off-main and immutable (`V2Frame.prepare` → `V2PreparedPage`): each
  page is converted ONCE into what CoreGraphics consumes — the exact `CTFont` per run
  (from the hash-resolved `CGFont`), `CGGlyph` IDs, absolute baseline origins in PDF
  space, rule `CGRect`s — on `V2Loader.queue` (serial, `userInitiated`), together with
  decoding, validation and font resolution. A frame is a value over immutable
  CoreFoundation fonts and carries a per-preparation nonce. Every load takes a
  monotonically increasing ticket; the result reaches the main run loop as a
  run-loop block plus wake-up (as `WorkerClient` delivers worker events) and is
  published only if its ticket is still the one the shell is waiting for — a
  superseded result is dropped and counted (`V2Loader.staleResultsDropped`). While a
  load is in flight the previous verified frame stays on screen with an explicit
  STALE indicator (header + page label, `v2-stale`), as the rendering-v2 proposal
  asks; a refusal drops it (nothing unverified stays visible).
- Drawing: one CoreGraphics routine (`GlyphRunRenderer.draw`) in PDF space (y up,
  1 unit = 1 pt) paints a prepared page's items in list order — rules as path fills,
  glyph runs with `CTFontDrawGlyphs` by ORIGINAL glyph ID at the absolute origins
  (advances are never re-added, no reshaping/kerning). `Export PDF (v2)…` calls it on
  a PDF context with one glyph per call (`glyphByGlyph`): CG's PDF writer otherwise
  merges glyphs into `Tj` strings positioned by the font's advances plus integer
  1/1000 em `TJ` adjustments, which drifted up to a pixel at line ends once the producer
  laid text out with TeX/TFM metrics (444 differing pixels per frame, now 0); the pane does not draw glyphs on the main thread at all: `V2PageRasterizer`
  (`@Observable`, bounded bytes, LRU) rasterizes each page once through
  `GlyphRunRenderer.rasterize` at the pane's pixels-per-point on its own queue and the
  canvas blits that bitmap 1:1 (device pixels) under the hover/caret overlays, so a
  hover or caret change costs a blit. Bitmaps of a frame that is no longer current are
  dropped on arrival and evicted on frame change (`staleBitmapsDropped`). Dark
  preview inverts paint in the bitmap only; export keeps the list's colors.
- Export/preview parity, tolerance 0 (`V2Parity`): every page's preview raster
  (the bitmap above) is compared byte-for-byte with the CG PDF export rasterized
  back by CoreGraphics into the same bitmap configuration (sRGB premultiplied RGBA,
  antialiased, font smoothing off, subpixel positioning on). Two measured causes of
  disagreement are fixed in the shared routine: CG's fast `fill(rect)` computes edge
  coverage differently from the scan converter that replays `re f` (one gray level
  along every rule row), so rules are path fills; and CG's PDF writer serializes
  numbers at 7 significant digits, so prepared coordinates and font sizes are
  quantized through the same `%.7g` (≤5e-5 pt from the tick geometry) and both
  sides start from identical numbers. Measured: 0 differing pixels at 1 and 2 px/pt
  on 22 real pipeline pages (text fixture, math+rules fixture, a 20-page/47k-glyph
  document) and at every scale tried on the two fixtures; at some fractional
  scales CoreGraphics rasterizes thin glyph stems differently through a `CTFont`
  than through the PDF-embedded font (e.g. one 0.7 px en dash at 1.37 px/pt), so the
  gate pins 1 and 2 px/pt (display scales) and reports other scales. Evidence and
  numbers: `docs/evidence/mac-preview-v2-parity-2026-09-12.md`.
  `FLASHTEX_V2_PARITY_OUT=<dir>` (and `FLASHTEX_V2_PARITY_SCALE`, default 2) makes the
  app write `parity.json`, `export.pdf` and per-page preview/export PNGs after each load.
- Hit/caret geometry (`V2Geometry`): click → the cluster whose `hit_rects` contain
  the point (half-open, in ticks, later-painted wins) → its `sources`; the shell
  checks the display list's document `sha256` against the current buffer and then
  goes through `ShellModel.navigate(to:expectedText:)` (same stale-revision refusal
  and rebase verification as v1). Synthetic clusters/rules report their
  `synthetic_reason`. Editor caret → clusters whose sources contain the byte: an
  exact caret bar when the cluster maps its bytes 1:1 and the compiler supplied a
  caret at that byte, else the whole cluster's rectangles (documented fallback;
  no width is divided by character count).
- Deviations between the schema and the pipeline that the model accepts, explicitly:
  `fonts[].format` is `opentype-cff` (Latin Modern) or `core14-afm` (metrics only,
  `byte_length` 0, never paintable) where the schema allows only `static-truetype`;
  `fonts[].sha256`/`font_id` is SHA-256(bytes ‖ face_index as u32 BE) (font-engine's
  `content_sha256`), so the store indexes both that and plain SHA-256(bytes); unknown
  JSON keys are ignored by `Codable` where the schema says `additionalProperties:
  false`; clusters must partition the run and source paths must name a declared
  document (stricter than the schema, as crates/rendering-core requires).
- Tests (`RenderingV2Tests`, `PreviewV2Tests`, `PreviewV2ShellTests`,
  `PreviewV2ParityTests`, `PreviewV2LiveTests`): real `flashtex-render --v2` fixtures
  (`Tests/FlashTeXMacTests/Fixtures/display-list-v2-*.json`: text and math from
  pipeline 7094ef7 with `apps/mac/Fonts` only; `display-list-v2-math-rules.json` from
  79ba728 with three typed fraction rules and Latin Modern Math) decode, resolve by
  hash and navigate ligature clusters (`ffi` = one glyph, three source bytes; `é`
  from `\'e`); the math fixture fails closed on the unbundled `latinmodern-math.otf`
  hash (the rules parity case resolves it from MacTeX's `lm-math` directory and skips
  without it); every fail-closed rule above has a negative case; the shared routine
  drawing a run built from CoreText's own glyph positions matches `CTLineDraw` with 0
  differing pixels; export equals preview with 0 differing pixels at the pinned
  scales; prepared geometry is verified against the tick geometry; stale load results
  and stale page bitmaps are dropped; retention is bounded; the fast reader equals
  Codable on every fixture and falls back on anything else; the live route against
  `Fixtures/fake_worker_v2.py` (a producer double that rebinds the real text envelope to
  each request): line routing, capability toggling, frame arrives/navigates/passes
  parity, second edit replaces it, old producer, per-request decline, failed result,
  stale/unsolicited/mismatched lines. Evidence:
  `docs/evidence/mac-preview-v2-latin-modern-2026-09-12.png`,
  `docs/evidence/mac-preview-v2-parity-2026-09-12.md`,
  `docs/evidence/mac-preview-v2-live-2026-09-12.md`.
- Not done: the rendering-v2 proposal's own `render_capabilities`/`render_format_selected`
  handshake (the live route negotiates per request through `layout_capabilities`
  instead); the preview-controller route; no clip/rotation/image primitives (rejected as
  unknown kinds/features); v1 → v2 caret sync uses the v2 clusters only while the pane
  is visible; the pipeline's own `--pdf` is still the legacy v1 writer, so parity is
  against the Mac CoreGraphics export (the product exporter is crates/pdf).

## Known upstream issue

`protocol/fixtures/compile-result.json` item text is `"Hello FlashTeX."` (15
bytes) but its source range is `0..<14` (`"Hello FlashTeX"`). The real compiler's
spans are exact. Reported to the fixture owner (Commander, FT-001).

## Not done

- The FT-002 compiler (`crates/compiler`, branch `agent/claude/compiler-foundation`
  at 29221d8 when checked) round-trips through this shell: `RealCompilerTests`
  runs only when `FLASHTEX_COMPILER` points at a built binary and verifies every
  emitted span slices back to its text after UTF-8→UTF-16 conversion. Without
  the binary that test is skipped and only the Python double is exercised.
- PDF export draws only what the contract's text items describe; it is not a
  TeX-engine PDF and has no compiler-produced `pdf_path` behind it. A result with
  zero pages exports one blank page (a PDF must have at least one).
- No image items. Typed rules exist only through negotiated `rules-v1`, which
  main's compiler does not yet accept (checked with `RealCompilerTests`). Caret
  sync highlights items and scrolls to the page under the caret, not to the
  item within the page.
- Screen capture of the running app was not possible from the agent's terminal
  (no Screen Recording permission); visual click behavior needs a human check.
