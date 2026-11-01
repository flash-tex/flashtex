# Capture conversion (drawing / photo → LaTeX) on the Mac

The Mac app ships exactly one model-backed feature: converting an iPad/Nearby
(or file-picker) capture into LaTeX for review. Every other AI integration was
removed (lane `mac-deslop`, branch `agent/mac-deslop/remove-assistant`) so the
editor stays unopinionated; third parties add their own features through the
helper-process contract in `docs/extensibility.md`.

The conversion itself runs inside `flashtex-bridge`; the Mac only selects the
provider, supplies its credential to that one child process, and keeps the
review/approval boundary: `capture_submit` → **Edit › Convert Capture**
(`capture_convert`, one vision request inside the bridge) → `capture_proposal`
queued in the review sheet → the existing prepare/verify/apply flow on explicit
approval. Nothing is inserted without the reviewer's Approve.

## Provider seam

`ConversionCredential.swift` defines `ConversionProvider` — today `none`
(conversion disabled; the bridge answers `provider_disabled`) and `xai` (the
bridge's `--enable-grok` path). A provider case carries its Keychain service
(`tech.jay3332.flashtex.ai.<provider>`), the bridge flag, the environment
variable the bridge reads the key from, its default/selectable model ids and
any legacy names. Adding a provider means adding a case here and teaching the
bridge its flag (see the crate-owner note at the end); the app never talks to
a provider itself.

Selection order (`ConversionCredential.provider`): `FLASHTEX_CONVERSION_PROVIDER`
in the environment (`none` / `xai`), else **Preferences (⌘,) → Capture
conversion → Provider** (`ConversionPreferences`, default `xai` so an existing
key keeps working). Model: `FLASHTEX_CONVERSION_MODEL` (the older
`FLASHTEX_GROK_CAPTURE_MODEL` / `FLASHTEX_GROK_MODEL` still count), else the
per-provider preference, else the provider's default.

## Supplying the key

Either of:

```sh
security add-generic-password -U -s tech.jay3332.flashtex.ai.xai -a key -w '<key>'
```

(`-U` replaces an existing item; remove with
`security delete-generic-password -s tech.jay3332.flashtex.ai.xai -a key`), or
**Preferences (⌘,) → Capture conversion → API key → Save to Keychain**. The
item is a generic password in the login keychain, written with
`SecItemAdd`/`SecItemUpdate` — no third-party code. An item stored by older
builds under `tech.jay3332.flashtex.xai` (accounts `xai` / `XAI_API_KEY`) is
migrated to the new service the first time it is read.

For one process only, the environment also works: `FLASHTEX_AI_API_KEY`
(checked first), then the provider's own names (`XAI_API_KEY`,
`FLASHTEX_GROK_API_KEY`). `FLASHTEX_KEYCHAIN_OFF=1` skips the Keychain
entirely (tests, headless runs). Source order is fixed: Keychain →
`FLASHTEX_AI_API_KEY` → provider names → none.

The key never appears in argv, JSON, logs, `captureNote`, status text or
evidence: every text derived from a resolution says only
`key present (Keychain)` / `key present (environment: XAI_API_KEY)` / absent.
The value leaves `ConversionCredential.Resolution` in exactly one place: the
bridge's environment (`inject(into:as:)` from `bridgeEnvironment`).

## Bridge launch

`attachDiscoveredBridge()` (auto-attach and Edit › Attach Capture Bridge)
calls `ShellModel.bridgeConversionLaunch()` once: with a provider selected and
a key resolved the bridge is launched with the provider's flag and its key
variable plus `FLASHTEX_GROK_MODEL` in an environment from which every
credential/token/proxy-like variable was first removed
(`ConversionCredential.bridgeEnvironment`); otherwise it is launched without
any provider flag (same stripping, no key). Bridge provider errors are shown
as guidance (`ShellModel.conversionFailureNote`): `provider_disabled` /
`provider_auth_missing` (choose a provider, add the key, re-attach),
`provider_auth_error` (401/403), `provider_rate_limited` (429),
`provider_timeout` (90 s), `provider_transport_error`. Re-attach the bridge
after changing the provider or key.

## Destination and wrapping (lane lane-capture-flow)

**The destination is the caret unless pinned.** When the iPad asks where to
insert, the Mac pins the caret for it in the bridge's caret mode
(`destination_pin.mode: "caret"`, transfer-v1 additive) under a stable
automatic id (`mac-caret-N`, `ShellModel.isAutomaticDestinationId`). That
anchor follows typing like a caret and is never invalidated by it, and the
Mac re-pins it at the caret when the iPad asks, when the capture arrives
(`forwardNearbyCapture` → `rebindAutomaticDestination`) and when it is
approved (`approveBridgeProposal`): the capture lands where the caret is when
**Insert at caret** is clicked, and an id the iPad still holds from an earlier
query or from before a bridge restart is re-bound, not refused. *Edit › Pin
Insertion Point* (⌘⌥P, `mac-anchor-N`, fixed mode) is the explicit override
with the strict contract: an edit that overlaps it drops it and Insert is
refused with `destination_reselection_required` — the note says what to do
(`insertionRefusalNote`), and the inspector's failed rows offer **Insert at
caret** (`recoverCaptureAtCaret`: re-pin the capture's own id at the caret,
resubmit from the inspector's bytes if the bridge never journaled it,
convert, insert after review).

**Wrapping happens at approval, from the caret's context**
(`CaretContext.wrapping(for:in:atByte:)`, sent as
`capture_prepare_insert.wrap {prefix, suffix, kind}` and journaled by the
bridge with the prepared edit; the applied text is
`prefix + proposal + suffix`, the proposal itself is never edited): a formula
on its own line → `\[ … \]` on its own lines; inside a sentence or a
tabular cell → `$ … $`; caret already in math → bare (a proposal carrying its
own delimiters there is refused with "move the caret"); already delimited or
an environment → never wrapped again (a block gets its own lines; display
math mid-sentence or in a cell is refused with "put the caret on its own
line"); `tikzpicture` → bare on its own lines, no automatic `figure`; text →
bare; verbatim/comment → literal. An undelimited `x = 2y + 1` counts as a
formula (`looksLikeFormula`, mirrored in the bridge's `caret::shape`). The
Captures inspector and the review sheet show exactly the wrapped text with an
"as display math / inline math / as is" indicator; the bridge gates the whole
replacement against the same context (`unsupported_construct_requires_confirmation`).
The bridge still normalises the proposal at conversion for the caret it was
received at (`caret::normalize`), so the two never disagree on delimiters.

Evidence: `CaptureFlowAcceptanceTests` (real bridge + real edit ledger + a
loopback OpenAI-compatible stub, no key, nothing leaves the machine): typing
after the iPad connected → inserted at the new caret; bridge restart with the
iPad's old id → accepted and inserted, TikZ on its own lines; explicit pin
overlapped → refused with the actionable note, Insert at caret recovers.

## `supported_features` (demo gate, issue #2)

`CaptureFeatures.swift` is the checked-in list sent with every
`capture_convert`: what the compiler renders at
origin/agent/claude/compiler-foundation `49e6eb43` (`COMMAND_GLYPHS` — 60
symbols, re-derived by `CaptureFeaturesTests` from `git show` when the commit
is present — plus `$…$`, `\[…\]`/`equation`, `\frac`, `\sqrt`, `^`/`_`,
`\left…\right`, `\section`/`\subsection`, `\textbf`/`\emph`/`\textit`,
`itemize`/`enumerate`, `\label`/`\ref`), an explicit NOT-supported list
(amsmath/amssymb environments, `\mathbb`, `\text`, `\bigl`, `\quad`, TikZ,
tables, macros), the OUTPUT rule (every formula wrapped in `$…$` or `\[…\]`)
and the RULE that unsupported constructs are reported in `ambiguities`, never
substituted. 20 entries, each ≤128 bytes (the bridge's limits).

## Tests

No network: `ConversionCredentialTests` (resolution order, legacy Keychain
migration, no text contains the key, a real `SecItem` round trip under a
throwaway service, provider/model selection and preference migration, bridge
environment stripping, and the bridge launch against `Fixtures/fake_bridge.py`
with and without a key); `PanelAccessibilityTests.testConversionPreferences…`
(the Preferences section's controls take keyboard focus);
`CaptureAcceptanceTests` / `RealBridgeTests` (the capture flow with the real
bridge and no provider); `CaptureFlowAcceptanceTests` (the caret destination
and wrap end to end with the real bridge, real ledger and a loopback provider
stub); `CaretContextTests` (the wrap rules).

Live, opt-in: `CaptureConversionLiveTests` runs one real conversion through
the real bridge only when `FLASHTEX_CONVERSION_LIVE=1` and a key resolves for
the selected provider; `FLASHTEX_CONVERSION_EVIDENCE_DIR` receives
`conversion.json` (provider, model, ids, byte counts, timings, the compile
gate — never a key, prompt or reply body). Earlier live evidence (2026-09-12,
`docs/evidence/grok-live-20260912T191209Z/`): the xAI non-reasoning model
converted a 900×260 handwriting PNG in 2.8 s and the proposal compiled with
zero diagnostics; the reasoning model hit the bridge's 90 s timeout, which is
why `grok-4.20-0309-non-reasoning` is the xAI default.

## Accessibility note

The Capture conversion section is shown by the app's `EditorPreferencesView()`
and hidden by `EditorPreferencesView(preferences:)` unless
`showConversion: true`, so the pinned Settings table
(`PanelFocusOrder.panels[0]`, checked against `EditorPreferences.swift`
markers) stays exact; its controls are keyboard-walked by
`PanelAccessibilityTests.testConversionPreferencesSectionControlsTakeKeyboardFocus`.

## Requests for the crate owners (crates are read-only for the Mac lanes)

- **`crates/bridge`:** rename `--enable-grok` to a neutral
  `--conversion-provider <name>` (plus `FLASHTEX_CONVERSION_MODEL` /
  `FLASHTEX_AI_API_KEY` read-through) so `ConversionProvider.bridgeFlag` and
  `bridgeKeyVariable` collapse to one rule; keep `--enable-grok` as an alias for
  one release. Carry provider evidence without bodies (response id, echoed
  model, usage) in `capture_proposal`.
- **`crates/assistant-context`:** the Mac no longer bundles or launches this
  helper (explanations, `--provider-session`, review/approve of model edits).
  Its explanation/provider paths and the `grok` feature can be deleted or kept
  as an external extension (`docs/extensibility.md`); the reviewed-proposal
  shapes the iPad reference view reads from `examples/review-workflow.json`
  are unaffected.
- **`crates/diagnostic-explanations` (`flashtex-explain`):** unchanged; it is
  rule-based (no model, no network) and stays bundled (`make-app.sh --explain`).
