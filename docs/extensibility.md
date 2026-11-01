# Extending FlashTeX with AI (or other) features

FlashTeX does not opinionate about AI. The editor ships exactly one
model-backed feature — converting an iPad/Nearby capture (drawing or photo)
into LaTeX for review, behind a provider-neutral seam
(`apps/mac/docs/capture-conversion.md`) — and nothing else: no assistant
sheet, no "explain with a model", no model-written quick fixes, no status
pill. Everything a third party wants to add (a chat assistant, a model-backed
explainer, a grammar checker, a citation finder, a different conversion
provider) plugs into the same three contracts the built-in helpers already
use. None of them require patching the app.

## 1. The helper-process contract (JSON Lines children)

Every non-trivial feature in FlashTeX is a separate executable the app
launches as a child process and talks to over private pipes with
newline-delimited JSON. The compiler, PDF writer, preview controller, edit
ledger, project-files indexer, capture bridge and the rule-based diagnostics
explainer all work this way; so did the removed assistant, and so can yours.

The shape (`apps/mac/Sources/FlashTeXMac/LineProcessClient.swift`):

- The app starts `<executable> <arguments...>` with a **sanitized
  environment**: every credential-, token- or proxy-like variable is stripped
  (`ConversionCredential.isSensitiveVariable`) and only the variables the
  feature declares are added back. A helper that needs a secret receives it in
  its environment only — never in argv, never in a JSON line.
- Requests are one JSON object per line on stdin, `{"id": "...", ...}`;
  replies are one JSON object per line on stdout, echoing `"id"` and carrying a
  `"type"` (or `{"id", "error": {"code", "message"}}`). stderr is diagnostics,
  never data. Lines are bounded (12 MiB either way; an oversized reply
  terminates the child).
- Replies are correlated by `id`; the app enforces a per-request timeout,
  outbound backpressure, and treats an unexpected reply type as a protocol
  violation. A child that exits is relaunched on the next request, never
  automatically in a loop.
- Discovery order for a bundled helper is fixed: an explicit environment
  variable (`FLASHTEX_<NAME>`), then the executable next to the app binary
  inside `FlashTeX.app/Contents/MacOS/`, then a checkout's own
  `crates/<crate>/target/{release,debug}/` build. `apps/mac/scripts/make-app.sh
  --<flag> <path>` copies a helper into the bundle and records its hash in
  `components.json`.

The runtime-v1 (`docs/contracts/runtime-v1.md`) and transfer-v1 contracts are
the reference wire shapes; `crates/diagnostic-explanations` (the
`flashtex-explain` front end) is the smallest complete example: one request
type in, one reply type out, no state, no network.

A third-party feature therefore is: an executable speaking this protocol, a
`FLASHTEX_<NAME>` variable (or a bundle copy), and a small Swift client that
subclasses nothing — it constructs a `LineProcessClient` with a `classify`
closure and enqueues lines. The app owns launch, timeouts and teardown.

## 2. The conversion-provider interface (the bridge)

Capture conversion lives in `crates/bridge` (`flashtex-bridge`). The Mac's
part is `ConversionProvider` (`apps/mac/Sources/FlashTeXMac/ConversionCredential.swift`):
a case per provider carrying

- the bridge flag that switches the provider on (`bridgeFlag`; today
  `--enable-grok` for `xai`),
- the environment variable the bridge reads the key from (`bridgeKeyVariable`),
- the Keychain service the key is stored under
  (`tech.jay3332.flashtex.ai.<provider>`, account `key`),
- default and selectable model ids, and any legacy environment names.

The user picks the provider in **Preferences → Capture conversion** (or
`FLASHTEX_CONVERSION_PROVIDER`), the model there (or
`FLASHTEX_CONVERSION_MODEL`) and stores the key in the Keychain (or
`FLASHTEX_AI_API_KEY`). `ShellModel.bridgeConversionLaunch()` resolves those
once at attach time and launches the bridge with the flag and the key in its
environment; `provider = .none` launches it with neither and every
`capture_convert` is refused with `provider_disabled`.

Adding a provider: add a case to `ConversionProvider`, and add the matching
flag/key handling to the bridge (`crates/bridge`, owned by the bridge lane —
the requested neutral `--conversion-provider <name>` flag would make the Mac
side a one-line change). The prompt the bridge sends carries the
compiler-derived `supported_features` list (`CaptureFeatures.swift`) so any
provider is told what the pipeline can typeset and must report the rest in
`ambiguities`.

## 3. The review / Apply gate

No extension writes into the editor. Every model output — the built-in
conversion included — reaches the document through the same reviewed path:

1. the helper returns a **proposal** (`capture_proposal` for conversions; for
   an edit-shaped feature, a `{path, start_byte, end_byte, removed_text,
   replacement}` list bound to the compile revision it was computed from);
2. the app shows it in the review sheet (`ProposalPreview`), shadow-compiles
   the document with the proposal inserted on a separate worker and lists the
   new diagnostics, without touching the live buffer;
3. the reviewer edits, rejects, or presses **Approve** — only then does the app
   turn the proposal into exactly one `ShellModel.PendingEdit`, applied through
   the text view's undo manager as one undoable step and, when an edit ledger
   is attached, committed durably through `capture_prepare_insert` /
   `apply_group` at the expected revision (a moved buffer refuses the insertion
   and returns the proposal to review with the reason).

The rule-based quick fixes (`EditorDiagnostics.QuickFix`, fed by
`flashtex-explain`) use the same `PendingEdit` path: preview, then one grouped
replacement, refused when the span was edited since the compile. An extension
that proposes edits should produce the same shape and let the app gate it;
one that only explains should return text and let the app display it as
"model output, not applied".

## What was removed, for orientation

Lane `mac-deslop` (2026-09-13) deleted from `apps/mac`: the "Ask Grok"
assistant sheet (⌘⌥G, toolbar, palette, `AccessibilityCommand.askGrok`),
"Fix with Grok" on Problems rows, the review sheet's model-backed **Explain**
(`ProposalPreview` explanation states, `GrokProviderSession`, `OneShotProcess`,
`AssistantRecoveryLog`), the Grok status pill, the Grok preferences section
and the connection probe, the `--assistant` helper bundling, and their tests
and fixtures. The Rust side (the former `crates/assistant-context`
explanation and provider-session library) was removed once nothing launched
it; a third party can still supply such a helper under contract 1. The iPad
companion keeps its bundled `review-workflow.json` fixture and the
reviewed-proposal contract it exercises.
