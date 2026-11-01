# Proposal: `caret_context` — the recogniser is told what kind of place the caret is in

Status: **PROPOSAL** (owner report after testing on a real iPad: "When necessary,
wrap in math (inline or display) blocks (or whatever is needed.) That is, the
model should also take in context regarding the location in the code." Issue #2,
lane `ipadmath`, branches `agent/claude/caret-context-*`). Additive and optional.
It changes no frozen shape: `protocol/rendering-v2.schema.json`, the
`compile_result` payload, `capture_submit`, `capture_proposal` and the
`destination` object of any peer that ignores the new key stay byte-for-byte as
they are.

Producer of the field: `apps/mac` (`CaretContext.swift`) for the nearby hop and
`crates/bridge` (`src/caret.rs`) for the provider hop. Consumers: the conversion
provider (prompt), `crates/bridge` (insertion), and the iPad companion (display
only). Co-sign needed from the `apps/ios` lane before the companion relies on
the nearby half (§6).

## 1. Summary

The destination pin says **where** a capture will be inserted. It has never said
**what kind of place that is**, so the recogniser had to guess, and it guesses by
wrapping: it returns `$x^2$` because that is what a formula usually looks like.
Inserted at a caret that is already inside `$…$`, that produces

```latex
Let $a + $x^2$ + b$ hold.
```

which pdflatex rejects outright. The same shape of failure appears for `\[…\]`
inside `equation`, and for display math inside a `tabular` cell.

1. A new object `caret_context` describes the caret's mode: text, inline math,
   display math, verbatim or a comment, plus the enclosing environments, whether
   amsmath is loaded, and a single derived `wrap` policy.
2. It rides on **two** hops, both additive and optional:
   - `destination_context.caret_context` in the conversion request the bridge
     sends a provider, so the recogniser wraps correctly in the first place;
   - `destination.caret_context` in nearby-v1 `hello_ack` / `destination_query`,
     so the companion can say where a capture will land before it is sent.
3. The **same value** decides the prompt and the insertion. A recogniser that
   ignores the instruction still cannot produce broken LaTeX, because the
   insertion re-wraps or refuses.
4. When absent, everything behaves exactly as it does today.

## 2. Why one value for both halves

Two independent mechanisms would drift: a prompt that says "you are in math" and
an insertion that wraps anyway is worse than either alone. `caret_context` is
derived once, from the snapshot the pin refers to, and used twice. `wrap` is
carried explicitly rather than re-derived by each consumer for the same reason.

## 3. Shape

```json
{"caret_context":{
  "mode":"text"|"inline_math"|"display_math"|"verbatim"|"comment",
  "delimiter":"$"|"$$"|"\\("|"\\["|"environment"|null,
  "environment":"align"|"tabular"|null,
  "environments":["document","tabular"],
  "amsmath":true,
  "wrap":"display"|"inline"|"already_math"|"literal"
}}
```

- `mode` — how the document reads at the caret.
- `delimiter` — what opened the current math, for a human reading the request.
  `null` outside math.
- `environment` — innermost open environment; `environments` is the enclosing
  stack, outermost first, capped at 16 entries.
- `amsmath` — whether the preamble loads amsmath. It decides `\text{…}` versus
  `\mbox{…}`; see §5.
- `wrap` — **authoritative**. The policy both the prompt and the insertion obey.

On the provider hop the object additionally carries `instruction`, one plain
English sentence built from the fields (§4). It is derived, not independent
state: a consumer may ignore it, and no consumer may contradict `wrap`.

Derivation is a **lexical scan**, not a TeX interpreter: a `$` produced by a
macro is invisible to it, exactly as it is to the Mac's syntax highlighter and
to the bridge's existing `definitions` context. That limitation is stated here
rather than hidden.

## 4. `wrap`, and what each value means

| `wrap` | When | The recogniser is told | The insertion enforces |
|---|---|---|---|
| `display` | Text mode, caret alone between blank lines | Wrap a standalone formula in `\[ … \]`, one inside a sentence in `$ … $` | Undelimited mathematics is wrapped in `\[ … \]` |
| `inline` | Text mode mid-sentence, or anywhere inside `tabular`/`longtable`/… | Wrap every formula in `$ … $`; display math is **not** legal here | Undelimited mathematics is wrapped in `$ … $`; a top-level display group is demoted to inline |
| `already_math` | Caret inside `$…$`, `$$…$$`, `\(…\)`, `\[…\]`, or a math environment | Emit bare mathematics with **no** delimiters | Outer delimiters are peeled; any remaining delimiter is a refusal |
| `literal` | Caret inside a verbatim-like environment or a comment | Emit the transcription with no LaTeX markup | Inserted exactly, with no wrapping and no newline padding |

Every row was checked against pdflatex rather than assumed;
`scripts/caret_context_oracle.py` compiles each case in
`protocol/fixtures/caret-context-v1.json` both ways and reports which rules turn
a real pdflatex error into a clean compile. At the time of writing, 13 of the 22
cases do.

**Note for implementers:** the FlashTeX engine is currently *more permissive*
than pdflatex on all of these — `flashtex-render` reports no diagnostic for
`Let $a + $x^2$ + b$ hold.`, for `\[…\]` inside `tabular`, or for `\text`
without amsmath. The engine therefore cannot be used to detect this class of
bug, and the oracle uses pdflatex. That divergence is worth a separate
diagnostics issue; it is not addressed here.

## 5. Prose at a math caret

Handwriting recognised at a math caret is sometimes prose, not a formula
("for all real numbers" written in the middle of an equation). Bare prose inside
math typesets in math italic with the spaces thrown away, so it must go in a
text box. The honest answer depends on the preamble:

- `\text{…}` — the idiomatic choice, and the one that spaces correctly. It is
  **amsmath**, and `$x + \text{y}$` without `\usepackage{amsmath}` is a pdflatex
  error (verified).
- `\mbox{…}` — the LaTeX kernel's, always available, correct spacing.

So: `\text{…}` when `caret_context.amsmath` is true, `\mbox{…}` otherwise, with
an `AMBIGUOUS: ` note either way so the reviewer sees the decision. This is the
only place the proposal adds markup the handwriting did not contain, which is
why it is announced rather than silent.

## 6. Migration gates

1. `crates/bridge` derives the field, sends it, and enforces it at insertion.
   A request that omits it behaves exactly as before (`serde(default)` yields
   text mode / `display`, today's implicit assumption).
2. `apps/mac` derives the same field for its own bridge-less insertion path and
   announces it with the destination. Its derivation is checked against
   `SyntaxHighlighter`, the app's existing authority on what is math, so the two
   cannot drift.
3. The two implementations are checked against **one** table,
   `protocol/fixtures/caret-context-v1.json`, by
   `crates/bridge/tests/caret_context.rs` and
   `apps/mac/Tests/FlashTeXMacTests/CaretContextTests.swift`. That table is in
   turn checked against pdflatex by `scripts/caret_context_oracle.py`.
4. **Not yet co-signed:** the `apps/ios` half. `NearbyWire.Destination` decodes
   the field and `ContentView` shows its label, but the one-screen capture flow
   (`CaptureView.swift`) is owned by another lane and its destination chip does
   not show it yet. That is the natural place for it and is left to that lane.
5. A producer that sends `caret_context` must not assume a provider honoured it.
   There is deliberately no echo field: the insertion re-checks the text it is
   about to write rather than trusting the reply.

## 7. Compatibility

- Old producer, new consumer: no `caret_context`; the consumer defaults to
  today's behaviour (text mode, `display`). No error, no change.
- New producer, old consumer: an unknown key. The bridge and
  `scripts/check_runtime.py` already tolerate unknown payload keys, and
  `NearbyWire` decodes into optionals, so an older companion ignores it.
- `protocol/fixtures/capture-submission.json` and the `capture_submit` /
  `capture_proposal` payloads are unchanged. `capture_submit` deliberately gains
  nothing: the companion does not derive this, the Mac does.
