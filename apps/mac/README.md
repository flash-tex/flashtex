# FlashTeX Mac shell (FT-003)

Native macOS source editor + preview shell. Swift Package, macOS 14+, SwiftUI/AppKit.
Owner: mac-claude-a. The preview's engine is `flashtex-host`, the
pdfLaTeX-compatible engine, run as a separate process and spoken to over the
display-list-v3 protocol (`docs/protocol/display-list-v3.md`); see
[docs/engine-v3-preview.md](docs/engine-v3-preview.md). Since the old-engine
retirement plan's stage S6 (#1236) it is the app's only engine: the old
worker, the preview controller (helper) route, the v1/v2 panes and their
commands are gone. The diagnostics shape is still runtime-v1's
(`docs/contracts/runtime-v1.md`).

Opened with no file, the window holds an untitled LaTeX document: File › New
Project…'s "Blank article" titled "Untitled" (`UntitledDocument` in
`ProjectScaffold.swift`), with the caret on the empty line under
`\section{Introduction}`. It is not dirty until edited, and the engine
compiles it like any other buffer.

A bare `ShellModel()` (tests, automation) seeds its documents, entry and
revision from `protocol/fixtures/compile-request.json` (`fixtureURL` is set;
`replaceProject` clears it). It shows no compile result: diagnostics and pages
come only from the engine, and under XCTest the engine session is off unless a
test turns it on (`EngineV3.enabledAtLaunch`).

## Build and test

```sh
cd apps/mac
swift build
swift test
xcodebuild -scheme FlashTeXMac -destination 'platform=macOS' build
FLASHTEX_REPO=$(git rev-parse --show-toplevel) .build/debug/FlashTeXMac
```

`scripts/run-mac-dev.sh` (repository root) builds `flashtex-host`,
`flashtex-bridge` and the app in release and launches it with the engine on.
The app locates `protocol/fixtures/` via `FLASHTEX_REPO`, the working
directory, the bundle path, or the source path. *File › Open Compile Result
Fixture…* and *Reload Fixture* were removed with the old engine's route (S6);
open a `.tex` file or a folder instead.

## Packaging

A bare `swift build` product has no Finder/Dock identity: `open -a` fails on it
and macOS cannot grant it per-app permissions (e.g. local network, later).
`scripts/make-app.sh` wraps the built executable in a minimal `FlashTeX.app`:

```sh
apps/mac/scripts/make-app.sh [--debug] [--bridge <path>] [--ledger <path>] [--project-files <path>]
  [--engine-host <path>] [--engine-host-unicode <path>] [--require-engine-host]
  [--open] [--install] [--dmg]
```

It builds `FlashTeXMac` (release by default), assembles
`apps/mac/build/FlashTeX.app` (`CFBundleIdentifier tech.jay3332.flashtex.mac`),
copies `protocol/fixtures/compile-{request,result}.json` and `Samples/*` into
`Contents/Resources/Samples`, bundles `flashtex-bridge`, `flashtex-edit-ledger`
and `flashtex-project-files` into `Contents/MacOS` when built (or passed via
`--bridge`/`--ledger`/`--project-files`; `scripts/ci/build-helpers.sh` builds
the three), stages the engine host as `Contents/Helpers/flashtex-host` with its
pool, licence and the TeX bundle's lock under `Contents/Resources/engine/`
(`--engine-host`; `--require-engine-host`, used by releases, fails the
packaging when it is missing) and Unicode mode's host as
`Contents/Helpers/flashtex-host-unicode` when built, and ad-hoc codesigns the
result. Launch with `open apps/mac/build/FlashTeX.app` or pass `--open`.
`apps/mac/build/` is gitignored. `--install` atomically replaces
`~/Applications/FlashTeX.app` (verified to launch before the previous bundle
is discarded) and `--dmg` produces a compressed disk image; see
`apps/mac/docs/packaging.md` for signing/notarization status and the
update-path and launch-recovery evidence (`scripts/launch-check.sh`, which
checks that the bundled engine host starts). `scripts/packaging-selftest.sh`
fails on a missing helper. The old engine's helpers (`flashtex-cli`,
`flashtex-compiler`, `flashtex-pdf`, `flashtex-render`, `flashtex-pdf-exact`),
the preview controller and `flashtex-explain` are no longer bundled, and
`--cli`/`--compiler`/`--pdf`/`--render`/`--pdf-exact`/`--controller`/`--explain`
are gone.

### Rooted TeX metrics (GH36)

`make-app.sh` still stages the five official LM 2.004 metrics (`ec-lmr10`,
`ec-lmr12`, `rm-lmr12`, `rm-lmr8`, `rm-lmr6`), the rooted GUST license and the
supplementary metrics (`apps/mac/Fonts/texmf/SUPPLEMENTARY-METRICS.json`)
under `Contents/Resources/texmf/…`:

- `scripts/bundle-texmf.py` (called by `make-app.sh`) verifies every file's
  byte length and SHA-256 against the Commander's pinned manifest
  (`apps/mac/scripts/native-assets-manifest.json`, itself
  SHA-pinned in `apps/mac/scripts/verify_bundle_resources.py`)
  BEFORE the build, stages them, then runs the pinned verifier over the whole
  `Resources` directory before signing. Any missing/mismatched/symlinked file
  refuses packaging; nothing is downloaded and the host TeX tree is never
  consulted. `FLASHTEX_BUNDLE_TEXMF_ROOT` points at another verified official
  root.
- The verified hashes are recorded in `Contents/Resources/components.json`
  (`"resources"`) and `Contents/Resources/resource-coverage.json`, both sealed by
  the app signature.
- They served the old engine's producer (`BundledMetrics.swift` appended them
  to `FLASHTEX_TFM_DIRS`; `scripts/texmf-acceptance.sh` checked the bundled
  producer without host TeX; evidence `docs/evidence/mac-bundle-texmf-<UTC>/`).
  Those left in S6; the app's engine reads the metrics of TeX Live or the TeX
  bundle. The staged files leave with `apps/mac/Fonts/` in stage S7 unless the
  TeX bundle reuses them (`docs/design/engine-v2/retirement-plan.md` §1.4).

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
  "missing — create" sidebar row, and TeX's ``LaTeX Error: File `x.tex' not
  found.`` (or `No file x.tex.` for an `\include`) gets a **Create x.tex**
  button on its Problems row (`MissingIncludeFix`, deterministic message
  parse). Context-menu
  *Rename…* moves the file, retargets the member's metadata and rewrites
  references in open documents (`ReferenceRewrite.plan`: one grouped
  `pendingEdit` per document, applied in sequence as the editor consumes each);
  *Delete…* detaches and `FileManager.trashItem`s. Both refuse the entry
  document and unsaved edits. Tests: `ProjectScaffoldTests`.
- Preview: the engine-v3 pane (`EngineV3Preview.swift`) draws the engine's
  display list exactly as Core Graphics draws the engine's PDF; clicking a
  glyph selects the source character it came from; see
  [docs/engine-v3-preview.md](docs/engine-v3-preview.md).
- Navigation converts the contract's zero-based end-exclusive UTF-8 byte offsets to
  a UTF-16 `NSRange` (`String.nsRange(utf8Bytes:)`). Offsets that are out of range,
  reversed, or inside a multi-byte scalar are rejected with a footer message rather
  than applied.
- Inline diagnostics: each diagnostic the engine reports with a `source` in the
  active document is underlined in the editor (red dotted for errors, orange
  for warnings); hovering shows the message and its notes (`EditorDiagnostics`).
  Marks are layout-manager temporary attributes, so undo and the text binding are
  unaffected. After edits a mark is rebased through `SourceMapping` or dropped when
  it overlaps the edited region — never drawn under the wrong text. Diagnostics
  with null `source` appear only in the Problems panel. The diagnostics list groups
  identical diagnostics (same severity and message) into one row with an
  "N places" menu that jumps to each
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
- Inline math preview on hover (`EngineV3MathHover.swift`, tests
  `EngineV3MathHoverTests`): resting the pointer over an inline formula shows
  it as already rendered on the page; nothing is rendered on the hover path.
- Dark preview toggle in the title bar (the pages drawn dark; images keep their
  pixels; exports are unaffected).
- Stale offsets are never applied: a span is rebased through the common
  prefix/suffix of the compiled vs current text (`SourceMapping`) and refused
  ("recompile to navigate") if it overlaps the edited region.
- Auto-compile (Settings > Compile, default on): every edit goes to the engine
  at once as byte edits; a newer compile supersedes the running one. The
  keystroke-to-page latency is in the status bar.

- Capture review and insertion (contract "Capture and insertion", Mac side):
  `Edit > Pin Insertion Point` (⌘⌥P) records the caret as a `destination_id`
  anchor (UTF-8 byte offset + revision + following context). `Edit > Open Capture
  Proposal…` (no shortcut) queues a `capture_proposal`; a review sheet shows editable
  LaTeX, the wrap and insertion preview, the destination, the ambiguities, and Approve / Reject (the shadow-compile preview of the
  proposal left with the old engine in S6). Approve applies exactly one edit
  through the text view's undo manager (⌘Z reverts). Repeated `capture_id`s never
  insert twice. If the buffer changed since pinning, the anchor is rebased by its
  context or, when the destination was deleted/ambiguous, reselection is required.
  See `Samples/capture-proposal.json`. No network or provider call is involved here.
- PDF export: `File > Export PDF…` (⌘⇧E) writes the PDF pdflatex would write
  for the editor's text, from the engine host's `export: true` run
  (`ExactPDFExport.swift`, `EngineV3Session.export`), published through
  `ExportSession.swift` (sibling temp file, overwrite-conflict check, atomic
  rename, Cancel). This is the app's only export route. `File > Print…` (⌘P)
  prints exactly those bytes through PDFKit's system print panel
  (`PrintController.swift`). `File > Print Source…` prints the editor buffer
  with line numbers from a copy and is disabled when no document is open.
  The old `flashtex-pdf-exact from-v2` export, and before it the CoreGraphics,
  `flashtex-pdf` and v2-pane exports, are gone.

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

`Samples/multipage-request.json` holds a two-page `main.tex` with non-ASCII
words ("naïve", "Résumé") so UTF-8 and UTF-16 offsets differ, and a missing
brace; `multipage-result.json` is the old engine's result for it. The app no
longer loads result files (S6): put the request's `main.tex` text in a file and
open it for manual click-to-source, caret-sync and diagnostics checks.
`Samples/README.md` describes the others.

- Opening another file while the buffer is dirty asks Save / Discard / Cancel;
  a discarded buffer stays recoverable for the session (Edit > Restore
  Discarded Buffer).
- The Problems panel is never hidden when diagnostics exist; each entry shows
  severity, message, notes and source.

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

The engine: `FLASHTEX_ENGINE_V3=0` keeps the engine session off (an editor
only); `=1` turns it on under XCTest. `FLASHTEX_HOST` / `FLASHTEX_HOST_UNICODE`
name the hosts, `FLASHTEX_V3_CACHE` moves the engine's cache
([docs/engine-v3-preview.md](docs/engine-v3-preview.md) lists the rest).
The app no longer reads `FLASHTEX_COMPILER`, `FLASHTEX_RENDER`,
`FLASHTEX_PDF_EXACT`, `FLASHTEX_PREVIEW_CONTROLLER`, `FLASHTEX_PREVIEW_V2` or
`FLASHTEX_LAYOUT_CAPABILITIES`, and the first launch removes the defaults
`FlashTeX.EngineV3.enabled`, `FlashTeX.EngineV3.legacyMigrated.v1`,
`FlashTeX.EngineV3.defaultEngine` and `FlashTeX.Preview.v1.debugStatus`.

`FLASHTEX_NO_ACTIVATE=1` launches without activating/focusing the window (for
automation; never steals keyboard focus).
`FLASHTEX_LOG=<path>` appends timestamped status lines for automation such as
`scripts/launch-check.sh`.
`FLASHTEX_AUTOATTACH=1` attaches the discovered capture bridge at launch (a
bridge bundled inside `FlashTeX.app` attaches by default; `=0` disables);
`FLASHTEX_SEED_FILE=<path.tex>` seeds the editor and `FLASHTEX_OPEN=<file or
folder>` opens a document at launch. Example (repository root):
```sh
scripts/run-mac-dev.sh apps/mac/Samples/recovery-demo.tex
```

## Completion and navigation

Package and class files (`.sty`/`.cls`: the `@`-as-letter mode, the kernel
vocabulary, macros "declared in mystyle.sty", go-to-definition into a
package input, the `\usepackage` hint for problems inside one) are
described in [docs/package-editing.md](docs/package-editing.md).

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
2. Commands from the bundled vocabulary `Sources/FlashTeXMac/Resources/
   supported-latex.json` (schema `flashtex-supported-latex/1`), generated by
   `scripts/gen-supported-latex.py` from the new engine's measured package
   support with pdflatex as the oracle (RQ5; see `supported-latex/README.md`).
   `render --install` writes the bundled copies and `check --installed` (CI,
   `gate.sh` and `make-app.sh`, which also copies it into
   `Contents/Resources`) refuses a stale one;
   `CompletionTests.testBundledInventoryMatchesTheGenerator` compares them.
   A command, class or package with no row is unknown, never unsupported.
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
step "Toggle Comment"). Editor ▸ Duplicate Line (⌥⇧↓) and Duplicate Line Up
(⌥⇧↑) copy every full line the selection touches below or above, leaving the
caret or selection on the copy, as one undo step (`EditorKeyHandling.duplicateLinesEdit`,
the Overleaf shortcut). Editor ▸ Move Line
Up/Down (⌥⌘↑ / ⌥⌘↓), Delete Line (⌃⌘K), Join Lines (⌃J), Sort Lines
Ascending/Descending (palette) and Trim Trailing Whitespace (palette) operate
on the full lines the selection touches as one undo step (`EditorLineCommands.swift`).
The delimiter pair around the caret is highlighted
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
- **Reveal Caret in Preview** (⌘⇧J): forward search — the engine-v3 pane
  scrolls to the caret's glyph and flashes it (the engine's source map,
  `EngineV3SourceMap.swift`). It is also the manual override for
  automatic following (`CaretFollow.swift`): it scrolls at once — no debounce,
  and regardless of the "Preview follows the caret" preference, on or off —
  and re-arms following (once the preference is on) after a manual preview
  scroll had stopped it. Automatic following itself re-arms the same way on a
  text edit, or on its own the moment the caret lands on a different source
  line or a different preview page (a caret move that stays within the same
  line does not re-arm it).

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| ⌘, | Settings window (editor preferences: font, wrapping, tab width, indent, appearance, auto-close brackets & math, completion list; Tab walks the controls top to bottom) |
| FlashTeX > Check for Updates… | Check for updates: asks GitHub Releases (`flash-tex/flashtex`, `/releases/latest`) for the newest version and shows installed vs available with the release notes; Download opens the release page in the browser, Skip This Version silences the background check for that tag. Nothing is downloaded or installed by the app (#694 slice 1). Settings > Updates > *Check for updates automatically* (off by default) checks once a day after launch and only speaks up when a newer release exists |
| ⌘O | Open LaTeX file… (becomes the `main.tex` entry document; the engine compiles it) |
| ⌘⌥N | New Project… sheet (folder, name, template: Blank article / Article with sections / Report with chapters / Homework sheet; writes `<folder>/<name>/main.tex` plus its `\input`/`\include` members, opens `main.tex` as the entry document with the include tree in the sidebar; asks before replacing existing template files) |
| ⌘N | New File… sheet (also the sidebar's + button and the project row's context menu): a rooted `.tex` name, subfolders allowed, never above the project root; "Insert `\input` at the caret" (on by default while the entry document is active) is one undoable edit; the file opens in a tab |
| File > Move To… | Move file: the active document into another folder of the project (also "Move to…" in a project row's context menu; dragging a row onto another row moves it into that row's folder, onto the tree's empty space to the project root). Every `\input`/`\include`/`\includegraphics`/`\bibliography`/`\addbibresource`/`\lstinputlisting` that resolved to the file is rewritten to the new rooted path — one undoable edit per open document (⌘Z there), closed documents of the include tree on disk; refused for the entry document, with unsaved edits, or onto an existing file |
| File > Project Fonts… | Project fonts sheet: the manifest's `[fonts]` table as four rows (Text, Math, Sans, Mono), each a searchable picker over the installed families the engine's index finds (Math lists the families with an OpenType MATH table) with a sample line in that family and "Class default" first; Apply writes the table into `flashtex.toml` (created from the template when absent; "Class default" everywhere without a manifest writes nothing) and the preview recompiles. The document's own `\setmainfont`/`\setmathfont`/`\setsansfont`/`\setmonofont` still win |
| ⌘S / ⌘⇧S | Save / Save As… (UTF-8; header shows "— edited" when dirty) |
| ⌘⌥R | Show in Finder: the active document's file, selected (also "Reveal in Finder" in a sidebar row's context menu; right-click the project tree's empty space for "Reveal Project in Finder") |
| Edit > Restore Discarded Buffer | Brings back the unsaved text replaced by a "Discard" decision when opening another file |
| ⌘B | Compile now (and restart the engine after it stopped; with auto-compile on, edits also compile as they are typed) |
| ⌘. | Stop compile: ends a compile that runs too long |
| ⌘⇧E | Export PDF… — the app's one export route: the PDF pdflatex would write for the editor's text, from the engine's export run. Progress and Cancel in the status bar; the file is written atomically, and a file changed on disk since you chose it is kept |
| ⌘P | Print… (the same bytes Export PDF… writes; system print panel; page size follows the PDF) |
| File > Print Source… | Print Source… (editor text with line numbers, monospaced, from a copy so the live editor is untouched) |
| ⌘⌥P | Pin insertion point at caret (capture destination anchor) |
| Edit > Open Capture Proposal… | Open capture proposal… file (review sheet; ⏎ approves, inserts one undoable edit; no shortcut since ⌘⇧I moved to the Captures inspector) |
| ⌘⇧I | Toggle Captures inspector (View; also the toolbar's Captures button): captures from the paired iPad with image, instruction and state (received → converting → proposal ready → inserted), the proposed LaTeX/TikZ, Insert at caret / Edit / Review… / Reject; opening it starts advertising and attaches the bridge; Pairing code… is one click |
| ⌘⇧U | Submit sample capture… (PNG/JPEG → `capture_submit` through the attached bridge) |
| ⌘⇧G | Convert capture (`capture_convert` for the latest received capture) |
| ⌘⇧N | Nearby Companion… (advertise, pairing code, paired devices, received captures; Return shows or resumes a pairing code, Esc cancels it or dismisses a banner, Tab walks Advertise → pairing controls → Forget → Clear; the step indicator, status row and every transition are VoiceOver text) |
| ⌘⇧P | Command palette (View; also the toolbar's Commands button): scope tabs for Files, Sections, Labels, Citations and every command in this table; type to filter, Tab cycles scopes, ↑/↓ choose, Return opens or runs, Esc closes |
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
| ⌘F | Find… (opens the source editor's find bar; AppKit's built-in incremental search) |
| ⌘⌥F | Find and Replace… (opens the find bar already showing its Replace row; a replacement is one undoable edit, so ⌘Z undoes it and the preview recompiles) |
| Edit > Find Next | Find Next (selects the next find-bar match; no key equivalent — ⇧⌘G is Convert Capture — Return in the find bar's search field does the same) |
| Edit > Find Previous | Find Previous (selects the previous find-bar match; no key equivalent for the same reason — Shift-Return in the find bar's search field does the same) |
| ⌘E | Use Selection for Find (sets the focused editor's selection as the find bar's search string) |
| ⌘J | Jump to Selection (scrolls the focused editor's current selection into view and centers it) |
| ⌘Z | Undo (including an approved capture insertion) |
| Esc / ⌃Space | Completion popup (supported commands, `\end{…}` for open environments, labels, citation keys, document words; never takes the keyboard from the editor) |
| ↑ / ↓ / Tab / ⇧Tab / Page Up / Page Down / Home / End / Return | Completion list keys, while the list is open: ↑/↓ or Tab/⇧Tab choose the candidate (wrapping; VoiceOver announces “n of m: candidate, kind, origin”), Page Up/Page Down move by a screenful of rows and Home/End go to the first/last row (never wrapping), Return/Enter inserts it over the typed token, Esc closes without inserting; typing narrows the list, any other caret move closes it |
| ⌘⇧Space | Signature help for the command whose argument the caret is in (also opens on `{`/`[` typed after a command name; `}`, Esc or leaving the argument closes it) |
| ⌘/ | Toggle `% ` line comment on the selection's lines |
| ⌘L | Go to line… (1-based line, line:column, or +N/−N relative to the caret; out-of-range numbers clamp; `:42` in the command palette jumps directly) |
| ⌥⇧↓ | Duplicate Line (every full line the selection touches, copy below, caret/selection stays on the copy, one undo step; the Overleaf shortcut) |
| ⌥⇧↑ | Duplicate Line Up (every full line the selection touches, copy above, caret/selection stays on the copy, one undo step) |
| ⌘⌥↑ / ⌘⌥↓ | Move line up / down (full lines only, no-op at the buffer edges; ⌥⌘[ / ⌥⌘] remain Previous/Next Occurrence) |
| ⌃⌘K | Delete Line (every full line the selection touches, one undo step) |
| ⌃J | Join Lines (one space; strips the next line's leading whitespace and a trailing `%` comment marker only when it ends the line) |
| Editor > Sort Lines Ascending | Sort Lines Ascending (stable, locale-aware compare of the touched lines; no key equivalent) |
| Editor > Sort Lines Descending | Sort Lines Descending (stable, locale-aware compare of the touched lines; no key equivalent) |
| Editor > Trim Trailing Whitespace | Trim Trailing Whitespace of the whole document (verbatim bodies and a line that is only `\\` plus spaces are left alone) |
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
| ⌘⇧B | Bold: wrap the selection in `\textbf{…}` (`\mathbf{…}` in math mode; one undoable edit, caret after the `}`, between the braces when nothing is selected). ⌘B is Compile |
| ⌘I | Emphasize: wrap the selection in `\emph{…}` (`\mathit{…}` in math mode) |
| ⌘U | Underline: wrap the selection in `\underline{…}` |
| ⌘⌥W | Wrap selection in command… (asks for a command name; common commands first, then the document's own macros) |
| ⌃⌘E | Change environment… (innermost pair; rewrites both `\begin` and `\end` names as one undo step; typing in either name updates the partner) |
| ⌃⌘T | Expand abbreviation or edit structure… (TeXpand: the grid editor inside a matrix/tabular/cases/align, the abbreviation prompt elsewhere; wraps a selection) |
| ⌥⇧R | Rename symbol: the `\label` key or user command under the caret across the open documents (Plan → Apply; one undoable edit per document) |
| ⌘⇧] / ⌘⇧[ | Next / previous diagnostic (refused if its span was edited since the compile) |
| ⌘⌥] / ⌘⌥[ | Next / previous occurrence within the diagnostics panel's selected group (wrapping; the row reads "k of n") |
| ⌘⌥C | Copy diagnostics as text (`path:line: error/warning: message` lines for the selected row, all when none; ⌘C while the list has the keyboard) |
| ⌘⇧J | Reveal caret in preview (scrolls the preview to the caret's glyph and flashes it) |
| Click preview text | Select its source (UTF-8 span → UTF-16; refused if edited since compile) |
| Help > FlashTeX Accessibility Help | Help window: focus order, what VoiceOver reads in each pane, every command above |

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
`/` and `:s` (literal, smart-case), no `:g`, `:'a`, `q` macros, `Ctrl-V`
block mode, `R` replace mode, or `.vimrc` mappings; `H`/`M`/`L` use the
visible rect without `scrolloff`.

Marks (`` `a ``, `'a`) and the jumplist follow the buffer for **every** edit,
not only Vim's own: `VimMode.adjustPositions` hangs off the editor storage's
`didProcessEditing` notification, so plain insert-mode typing, ⌫, an input
method committing, completion, paste, undo/redo and a programmatic
replacement all move them. Text deleted out from under a mark unsets it
(`` `a `` then answers `E20: Mark not set`) rather than relocating it
silently, while a jumplist entry collapses to the edit instead — Vim's own
`ONE_ADJUST` / `ONE_ADJUST_NODEL` split, measured against vim 9.1. Positions
are UTF-16 offsets rather than Vim's line + column, so an edit before a mark
on the *same* line moves it here where Vim would leave the column alone.

## Targets

- `FlashTeXProtocol` — Codable models for runtime v1 (`RuntimeV1`, the
  diagnostics shape) and the capture bridge (`TransferV1`), byte-offset
  conversion, changed-region diffing.
- `FlashTeXDisplayListV3` — the display-list-v3 protocol and decoder (MIT,
  Foundation only); `FlashTeXPreviewV3` — its Core Graphics renderer.
- `FlashTeXMac` — the app. The engine session (`EngineV3Session`,
  `EngineV3Host`), `BridgeClient` (JSON Lines transport, id-correlated
  replies, 12 MiB line limit), `BridgeSession` (bridge-side document shadow,
  destination, captures, edit ledger, reconciliation), `ShellModel+Bridge`,
  `LineProcessClient` (shared bounded JSON Lines process transport) and
  `EditLedgerClient` (edit-ledger helper protocol).
- Tests: `swift test`. `RealBridgeTests` and `RealEditLedgerTests` are gated on
  `FLASHTEX_BRIDGE` and `FLASHTEX_EDIT_LEDGER`; the engine's end-to-end tests
  need a built `flashtex-host` and TeX Live (see
  [docs/engine-v3-preview.md](docs/engine-v3-preview.md) › Gates). The Python
  doubles in `Tests/FlashTeXMacTests/Fixtures/` are `fake_bridge.py`,
  `fake_edit_ledger.py` and `fake_project_files.py`; the old engine's doubles
  and display-list fixtures (`fake_worker.py`, `fake_worker_v2.py`,
  `fake_preview_controller.py`, `display-list-v2-*.json`) were deleted in S6
  with the tests that used them.

## Removed in retirement stage S6

The old engine's route and everything only it offered: the worker transport
and *Attach Built Compiler* (⌘⇧K), *Attach Render Pipeline* (⌘⇧R), *Attach
Worker Executable…* (⌘K) and *Detach Worker*; *Open Compile Result Fixture…*
(⌘⇧O) and *Reload Fixture*; *Open Display List (v2)…* and the v1 and v2
preview panes; the negotiated layout capabilities (`rules-v1`,
`font-hints-v1`, `display-list-v2`); the preview-controller (helper) route
and what it powered (*Find in Project…* ⌘⇧F with ⌘G, *Edit › Rename
Citation…*, *Edit › Durable History…*, `\cite`/macro navigation through the
project index, ledger-grouped renames), the helper display candidates and the
historical preview; the per-document engine choice (the status bar's engine
item, *View › Engine for This Document*, *Settings › Compile › Engine for
other documents*), its fallback rules and banners, and the Unicode-mode
suggestion banner; the `flashtex-pdf-exact` export; and the capture review
sheet's shadow-compile preview. Their history is in the retirement plan
(`docs/design/engine-v2/retirement-plan.md`) and `docs/evidence/`.

## Not done

- Screen capture of the running app was not possible from the agent's terminal
  (no Screen Recording permission); visual click behavior needs a human check.
- Engine gaps are listed in [docs/engine-v3-preview.md](docs/engine-v3-preview.md) › Known gaps.
