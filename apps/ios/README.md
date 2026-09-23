# FlashTeXPad — iPad capture companion for the Mac app

Owner: lanes `mac-ios-app` (acceptance slice) and `mac-ios-app-2` (finish:
outcome status, QR pairing, persistence) — parent `mac-claude-a`, machine `mac-m1max-a`.
Issue #51 / issue #2 comment 5647841253, re-centred per the user's scope
correction: the iPad is the **capture companion** (FT-004's original idea) —
draw with Apple Pencil or photograph a sketch/matrix, add an instruction, send
it to the Mac, and the Mac's bridge converts it into a reviewed LaTeX/TikZ
proposal that the Mac user approves. iPad **simulator only**; no device run.

## What is real vs. not carried (truthful table)

| Flow | Real on the wire? | Contract | Where |
|---|---|---|---|
| Pairing with the Mac's code (HKDF → TLS-PSK → `hello`/`hello_ack`, `pair_psk` stored) | **Yes** | nearby-v1 §2/§4 | `Packages/FlashTeXPadKit/Sources/NearbyClient` (symlink to `apps/mac/tools/nearby-client`), `MacLink.swift` |
| Current pinned destination (`hello_ack.destination`, `destination_query`) | **Yes** | nearby-v1 §4 | `MacLink.swift`, shown on the Capture screen |
| Apple Pencil canvas (PencilKit `PKCanvasView`, tool picker, finger allowed) → PNG | local | — | `CaptureView.swift` (`PencilCanvas`) |
| Photo picker (`PhotosPicker`) and bundled sample image (`sample-capture.png`) → PNG | local | — | `CaptureView.swift` (camera does not exist in the simulator) |
| Instruction text (≤ 4096 bytes) | local | transfer-v1 `capture_submit.instructions` | `CaptureQueue.validate` |
| **Send to Mac**: `capture_submit {capture_id, destination_id, base_revision, image{png}, instructions}` → `capture_received {capture_id, durable, has_proposal, applied}` | **Yes** | transfer-v1 over nearby-v1, exactly as the reference client | `CaptureQueue.send` |
| Per-capture status list: drafted → sending → received (durable / not) · refused `{code}` · not acknowledged (retry same `capture_id`) · discarded | **Yes** (from the receipt / error) | nearby-v1 §4, §8 | `CaptureQueue.swift`, `CapturesList` |
| Duplicate/retry: after a disconnect the same `capture_id` + payload is re-sent; the Mac de-duplicates | **Yes** | nearby-v1 §4 | `CaptureQueue.send` (`attempt` counter), test `testRetryAfterDisconnectReusesCaptureID` |
| Conversion progress after the receipt — journaled → converting → proposal ready → inserted / rejected / failed — and the returned LaTeX/TikZ text (read-only) | **Yes** — additive `capture_status` → `capture_status_ack` on the same authenticated session (nearby-v1 §4 / §6a, this lane: Mac listener + `ShellModel+Nearby.swift` + reference client), polled every 2 s after `capture_received` until final; a Mac that predates the message answers `unknown_type` and the row says "outcome unavailable" | nearby-v1 §6a | `CaptureQueue.refreshOutcome`, `PadModel.pollOutcome`, `CapturesList` (`capture.outcome.<id>`, `capture.latex.<id>`) |
| Recovery when the Mac's per-pairing acknowledgement memory lost the id (`unknown_capture`) | **Yes** — the iPad re-delivers the saved envelope (same capture_id / destination / base_revision / bytes; the Mac and bridge de-duplicate) and asks again | nearby-v1 §6a | `CaptureQueue.redeliver`, test `testUnknownCaptureAfterMacRestartIsRecoveredByRedelivery` |
| QR pairing | **Yes** for the payload path: VisionKit `DataScannerViewController` on hardware that supports it, paste-the-URL fallback everywhere; the Mac is found by Bonjour `fp`, or by typed host/port (simulator: no camera, no advertising listener in tests) | Mac `PairingQR.swift` payload `flashtex-nearby://pair?v=1&code&salt&fp&name` parsed by the reference client's `NearbyBootstrapPayload` (fp must derive from salt) | `PairingScanner.swift`, `MacLinkPanel` "Pair by QR", `PadModel.pair(bootstrapText:host:port:)` |
| Persistence | **Yes** — pairing in the Keychain (`kSecClassGenericPassword`, one item per Mac fp, `ThisDeviceOnly`); drafts (PNG as sent) + receipts + outcomes in `Application Support/FlashTeXPad/captures.json` + `captures/<id>.png`; restored on relaunch (an interrupted send comes back retryable with the same id) | — | `PadStore.swift` (`KeychainPairStore`, `CaptureStore`), tests `testPairingSurvivesInTheKeychainAndCapturesOnDisk`, UI `testRelaunchRestoresCapturesAndPairing` |
| **LaTeX editor** (lane `lane-ipad-editor`): syntax colouring, auto-close with type-over and pair deletion, the Return key's environment rules (`\item ` templates, `\end{…}`), bracket/`$` match highlight, completion from the compiler's inventory with one-line docs and Tab-stop snippets, hardware key commands (⌃Space, Esc, Tab, ⌘/, ⌘] ⌘[, ⌘⇧B, ⌘I, ⌘E), an accessory bar over the on-screen keyboard | local — the same rules as the Mac editor, from the shared `FlashTeXEditorCore` target (`apps/mac/Sources/FlashTeXEditorCore`, symlinked into `FlashTeXPadKit`) | the bundled `Resources/supported-latex.json` is byte-identical to the Mac's and the compiler's (`apps/mac/scripts/sync-supported-latex.sh --check`, CI) | `EditorController.swift` (delegate: auto-close, Return, match, commands, accessory bar), `EditorTextStorage.swift` (incremental colouring), `EditorTheme.swift`, `EditorAccessoryBar.swift`, `EditorView.swift`; `FlashTeXPadKit/LocalCompletion.swift`; tests `EditorControllerTests`, `EditorHighlightTests`, `EditorCompletionTests` |
| Diagnostics / reviewed-proposal gate | local, **reference only** | runtime-v1 / assistant-context shapes | sidebar section "Reference (Mac contracts; not the product)"; tested, small, not the product |

What the iPad still cannot do: approve, reject or edit the proposal (the Mac
user does; `latex` is read-only here), receive a push from the Mac (the iPad
polls), run the conversion itself, or compile: the editor edits a local `.tex`
buffer (opened from Files or the bundled `demo.tex`) and does not sync it to
the Mac — transfer-v1 carries captures, not documents.

## The editor

Measured in the simulator's hosted unit tests (Debug build, iPad Air 11-inch
(M4) simulator, 200 KB generated document; the numbers are printed by the tests):

| Budget | Measured | Test |
|---|---|---|
| `PadModel.textChanged` on the main actor per keystroke, 200 KB document, < 1 ms | median 0.007 ms, max 0.013 ms | `EditorCompletionTests.testTextChangedOn200KBDocumentReturnsUnderOneMillisecond` |
| Syntax recolouring per keystroke on the main actor, 200 KB document, < 4 ms | median 0.75 ms (storage only; one line re-lexed) — whole keystroke through the controller and `UITextView`, no window: median 1.8 ms | `EditorHighlightTests.testKeystrokeOn200KBDocumentStaysUnderBudget`, `…testControllerKeystrokeOnLargeDocumentReportsWithinBudget` |

Completion runs off the main actor from a text snapshot after a 120 ms
debounce (`PadModel.scheduleCompletions`); a newer edit cancels a pass in
flight. The colouring is an `NSTextStorage` subclass whose `processEditing`
re-lexes only the edited lines with the shared `SyntaxHighlighter` line table
(the incremental invariant — an edit's result equals a fresh lex — is
`EditorHighlightTests.testIncrementalEditsEqualAFreshLex`). One trap worth
knowing: `NSTextStorage`'s default `fixAttributes(in:)` walks every attribute
run of the whole string whatever range it is given (85 ms per keystroke on
200 KB, 58,000 `attributes(at:)` calls); the subclass fixes the range on its
backing store instead.

## Layout

```
apps/ios/
  FlashTeXPad.xcodeproj/            generated — do not hand-edit
  scripts/generate-xcodeproj.py     deterministic pbxproj + scheme writer (stdlib only)
  Packages/FlashTeXPadKit/          SwiftPM package (iOS 17 / macOS 14)
    Sources/FlashTeXProtocol  -> ../../../../mac/Sources/FlashTeXProtocol            (symlink, read-only reuse)
    Sources/NearbyClient      -> ../../../../mac/tools/nearby-client/Sources/NearbyClient (symlink, read-only reuse)
    Sources/FlashTeXEditorCore -> ../../../../mac/Sources/FlashTeXEditorCore         (symlink; the shared editor logic — owned by both apps, tested by both)
    Sources/FlashTeXPadKit/   CaptureQueue (product: send, outcome polling, re-delivery), MacLink, PadStore (Keychain pairings, on-disk captures), PadDocument, ReviewedProposal, ReviewSession, LocalCompletion, Diagnostics
  FlashTeXPad/                      SwiftUI app: Capture (primary), Mac link, reference .tex panels
    CaptureView.swift               PencilKit canvas, PhotosPicker, sample image, instruction, Prepare/Discard/Send, status + outcome list (LaTeX read-only)
    PairingScanner.swift            VisionKit DataScannerViewController wrapper for the Mac's pairing QR (paste fallback in ContentView)
    Resources/sample-capture.png    320×240 sketch (triangle with a right-angle mark), generated
    Resources/demo.tex, review-workflow.json, compile-result.json   reference-panel fixtures
  FlashTeXPadTests/                 XCTest hosted in the app: FakeMac (+ scripted capture_status) + CaptureQueueTests + FinishTests + AcceptanceSliceTests
  FlashTeXPadUITests/               XCUITest: CaptureFlowUITests (runner hosts FakeMac), FlashTeXPadUITests (.tex reference)
```

## Build and test (Xcode 26.3, iOS 26.3 simulator runtime)

```
cd apps/ios
python3 scripts/generate-xcodeproj.py       # only after adding/removing source files
xcodebuild -project FlashTeXPad.xcodeproj -scheme FlashTeXPad \
  -destination 'platform=iOS Simulator,name=iPad Air 11-inch (M3)' build
xcodebuild -project FlashTeXPad.xcodeproj -scheme FlashTeXPad \
  -destination 'platform=iOS Simulator,name=iPad Air 11-inch (M3)' test
```

`xcrun simctl list devices available | grep iPad` lists the names this Xcode
offers. Evidence for the recorded run: `docs/evidence/ios-acceptance-2026-09-12/`.

Automation hooks (launch arguments, no network unless given):
`-flashtexpad-test-mac host:port:saltHex:fp:code` pairs with a listener at
launch; `-flashtexpad-open sample|fixture` opens a reference document;
`-flashtexpad-fresh` wipes the Keychain pairing and the on-disk captures first.

## Running it on a real iPad (not done in this lane — simulator only)

The project is generated with `CODE_SIGN_IDENTITY=-` and no team, so a device
build needs signing set once:

1. `cd apps/ios && python3 scripts/generate-xcodeproj.py` (if sources changed), then
   `open FlashTeXPad.xcodeproj`.
2. Target **FlashTeXPad** > Signing & Capabilities: tick *Automatically manage
   signing*, pick your Apple ID team; bundle id is
   `tech.jay3332.flashtex.FlashTeXPad` (`BUNDLE_PREFIX` in the generator — change
   it if the id is taken in your team). No extra capability is required: the app
   uses the local network (`NSLocalNetworkUsageDescription`, `NSBonjourServices`
   `_flashtex._tcp` are already in `Info.plist`), the camera for QR scanning
   (`NSCameraUsageDescription` is in `Info.plist`; the simulator never asks),
   and the Keychain (default app access group, no
   entitlement needed). Photos come through `PhotosPicker` (no permission prompt).
3. Command line equivalent, with your team id:
   `xcodebuild -project FlashTeXPad.xcodeproj -scheme FlashTeXPad -destination 'generic/platform=iOS' -allowProvisioningUpdates DEVELOPMENT_TEAM=<TEAMID> CODE_SIGN_STYLE=Automatic CODE_SIGN_IDENTITY="Apple Development" build`
   then install with Xcode (Product > Run on the connected iPad) or
   `xcrun devicectl device install app --device <udid> <path to FlashTeXPad.app>`.
   A free Apple ID works (7-day profile); the first launch needs *Settings >
   General > VPN & Device Management* trust for a personal team.
4. Pair with the Mac app (`apps/mac`, `scripts/make-app.sh` or `swift run`):
   Mac *View > Toggle Captures* (⌘⇧I; advertising starts, the bridge attaches)
   > *Pairing code…* — the Nearby window shows the 6-digit code, the QR, and
   host:port. On the iPad, *Mac link* > *Find nearby Macs* and tap the Mac with
   the code typed, or *Scan QR…* (or paste the `flashtex-nearby://pair?…`
   text); if Bonjour does not find the Mac (different subnet, AP isolation),
   type the host and port shown on the Mac before tapping *Pair from payload*.
   Allow the Local Network prompt on both sides. Then put the caret where the
   result goes on the Mac (no pin needed; ⌘⌥P overrides), draw or tap *Camera*
   on the iPad, pick an instruction chip, tap *Send*; the Mac's Captures
   inspector shows the row, converts it (a conversion provider key, see
   `apps/mac/docs/capture-conversion.md`) and *Insert at caret* inserts it; the
   iPad row follows journaled → converting → proposal ready (LaTeX shown) →
   "Inserted on Mac ✓". Relaunching the iPad reconnects to the stored Mac by
   itself (remembered address, then Bonjour by fingerprint).

## The proofs (all in the simulator)

Mac-side fixture: `FakeMac` — a copy of the reference client's test fixture
(same TLS-PSK parameters as `NearbyListener`, verifies `hello.proof`, issues
`pair_psk` on the bootstrap key, answers `destination_query` and
`capture_submit` with an in-memory `durable:false` receipt, errors on anything
else; loopback only; no conversion provider, no Rust helper). It is hosted in
the test process (unit tests) or in the XCUITest runner (UI tests).

- `CaptureFlowUITests.testDrawSendReceipt` — app pairs with the runner's
  FakeMac at launch; three finger drags draw a triangle on the PencilKit
  canvas; instruction "Convert this triangle to TikZ"; Prepare → Send; the
  list shows "received — Mac inbox…"; the runner asserts the Mac got one
  `capture_submit` with a structurally valid PNG (`NearbyWire.checkImage`),
  the instruction, `destination_id dest-tikz`, `base_revision 3`.
- `CaptureFlowUITests.testDiscardBeforeSendSendsNothing` — draw, Prepare,
  Discard → status "discarded before sending", the Mac received nothing.
- `CaptureFlowUITests.testSampleImagePath` — bundled sample image → Prepare →
  Send → receipt; PNG valid on the Mac side.
- `CaptureQueueTests` (hosted): PencilKit drawing → PNG → sent bytes arrive
  unchanged with the instruction and destination; discarded draft never
  leaves; retry after a dropped connection reconnects with the stored
  `pair_psk` and re-sends the same `capture_id` (Mac sees it once); local
  refusal of a bad image / oversized instruction; no pinned destination →
  refused, nothing sent.
- `AcceptanceSliceTests` / `FlashTeXPadUITests` — the first-iteration `.tex`
  reference flow (open sample, pair, capture receipt, cancel inserts nothing,
  approve inserts exactly once with the receipt echoed).

## Gaps (honest)

- Signing: `CODE_SIGN_IDENTITY=-`, no team; simulator only in this lane. See
  "Running it on a real iPad" above.
- The QR *camera* path (VisionKit) is exercised only by its availability check
  in the simulator (`DataScannerViewController.isSupported == false` there); the
  payload → pairing path is tested. Bonjour discovery by fp is real code
  (`NearbyBrowser`) but the tests use typed host/port (no advertising listener).
- The iPad polls (2 s) — no Mac → iPad push; polling stops at a final state,
  after 150 polls (5 min) per capture, or when the Mac cannot answer.
- No approve/reject from the iPad; no App Store packaging; iPad only
  (`TARGETED_DEVICE_FAMILY=2`); no end-to-end run against the real Mac app + Rust
  bridge from the iPad in this lane (Mac side of `capture_status` is covered by
  `swift test --filter "Nearby|Pairing|CaptureAcceptance"` in `apps/mac`).
