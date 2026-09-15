# Proposal: structured labels, notes, and help on runtime-v1 diagnostics

Status: **PROPOSED** (issue #277 parts (1) and (3); lane
`agent/daniel-parent/structured-diagnostics`). Additive and optional. It changes
no frozen shape: `docs/contracts/runtime-v1.md`, `docs/contracts/runtime-v1-*.md`,
`protocol/*.schema.json`, and `protocol/fixtures/compile-result.json` stay as they
are. A diagnostic that omits the new fields is byte-for-byte what it is today.

Part (2) of #277 is the CLI renderer (PR #303,
`crates/flashtex-cli/src/report.rs`). Part (4) is the Mac Problems panel
(`EditorDiagnostics.QuickFix`). This document is the wire contract those
consumers read once a producer emits the fields.

## 1. Summary

Today a runtime-v1 diagnostic is `{severity, message, source, recovery}` plus the
issue-#76 optional fields `code` and `suggestion` (absent, never null, when
unset). `flashtex check` therefore prints one line, and the Mac Problems panel
has a message, a source range, and a Fix… affordance that is not driven by the
compiler.

The target report is rustc-style: a one-line message, carets with **labels**,
`= note:` / `= help:` lines, and an optional mechanical replacement the app can
apply. That needs three new optional fields on each diagnostic, and nothing else:

1. `labels` — extra underlined spans with caption text (primary + secondary).
2. `notes` — extra `= note:` strings; not recovery, not the suggested fix.
3. `help` — the suggested fix (`= help:`), with an optional
   `replacement` (`source` + `text`) the Mac Fix… button can apply.

`message` stays a **single line** and still contains **no recovery text**.
Recovery remains the existing nullable `recovery` field. Old consumers that only
read `severity` / `message` / `source` / `recovery` (and optionally `code` /
`suggestion`) ignore the new keys.

## 2. Why these shapes

runtime-v1 already has a span type used by diagnostic `source` and by every
page item: `{path, start_byte, end_byte}` — zero-based, end-exclusive UTF-8 byte
offsets into the named document of the request (`docs/contracts/runtime-v1.md`).
Labels reuse that object as a nested `source` so a secondary label can point at
a different range or file (the `\begin` that does not match an `\end`; the
`\label` nearest an undefined `\ref`) without inventing a second coordinate
system.

`help.replacement` uses the **same `source` object** as `labels[].source`
(`{path, start_byte, end_byte}`) plus `text`. A replacement can target a
different document than the diagnostic's own `source` — for example an
`\input` that should be edited in the included file, or a `\ref` whose
fix lives next to a `\label` in another path. Emitting `path` on the
replacement, rather than implying the diagnostic's path, is what makes
that legal without a second coordinate system. `EditorDiagnostics.QuickFix`
already rebases `{path, start_byte, end_byte, text}` from compiled bytes
onto the current editor text; today's QuickFix may still refuse
`.otherDocument` until part (4) grows a multi-file edit.

The existing `suggestion` field (issue #76) stays. It is replacement **text for
the diagnostic's entire `source` range** — currently a did-you-mean command such
as `\alpha` on `unknown_command`. `help.replacement` may cover a **different**
range (wrap `\tilde{c}_t` in `\(...\)`). Both may be set; consumers prefer
`help.replacement` when present and otherwise keep using `suggestion` as they
do today.

## 3. Wire format

Each object in `payload.diagnostics` may gain these keys. They are **omitted
when unset**. They are never `null`, never `[]`, never `{}`. A producer that has
nothing to say writes today's object.

```json
{
  "severity": "error",
  "code": "unsupported_feature",
  "message": "`\\tilde` is not implemented in text mode",
  "source": {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240},
  "labels": [
    {
      "source": {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240},
      "text": "this command",
      "primary": true
    }
  ],
  "notes": ["\\tilde is a math accent; here it is outside math mode"],
  "help": {
    "message": "wrap it in math: \\(\\tilde{c}_t\\)",
    "replacement": {
      "source": {"path": "notes.tex", "start_byte": 1234, "end_byte": 1240},
      "text": "\\(\\tilde{c}_t\\)"
    }
  },
  "recovery": "the command was skipped and its argument typeset as text"
}
```

The compiler's JSON writer stores objects in a `BTreeMap`, so when the new keys
are present they appear in alphabetical order among the existing keys
(`code`, `help`, `labels`, `message`, `notes`, `recovery`, `severity`, `source`,
`suggestion`). When they are absent the bytes of that diagnostic are unchanged.

### 3.1 `labels`

Array of one or more objects:

| field | type | rule |
|---|---|---|
| `source` | `{path, start_byte, end_byte}` | Same meaning as diagnostic `source`. `path` is a project-relative document in the request. Offsets are UTF-8, zero-based, end-exclusive. An empty range (`start_byte == end_byte`) is a caret at that point. |
| `text` | string | Caption drawn next to the carets (`this command`). May be empty only when the underline itself is the label; prefer a short phrase. |
| `primary` | bool | Exactly one element of a non-empty `labels` array has `primary: true`. The rest are secondary. |

When `labels` is omitted, the diagnostic's `source` is the implicit primary span
and there is no caption. Producers that emit `labels` should keep the primary
label's `source` equal to the diagnostic `source` (or a subrange of it) so old
consumers that only underline `source` still point at the right bytes.

A secondary label on another file is legal. Renderers that show one excerpt
(the CLI today) print the primary range and may list other labels as extra
`= note:`-style lines until they grow multi-file excerpts.

Malformed `source` (unknown path, `end_byte < start_byte`, offsets past the
document) is the producer's bug; consumers clamp like the CLI already does for
the diagnostic span and do not fail the compile.

### 3.2 `notes`

Array of one or more non-empty strings. Each is one `= note:` line. Wrapping is
the renderer's job (CLI #303 wraps at 80 columns); the string itself is not
required to contain newlines and should not contain the `= note:` prefix.

Notes are for extra facts (`\\tilde is a math accent; here it is outside math
mode`, the list of paths `\input` tried). They are not the suggested fix and
not the recovery.

### 3.3 `help`

A single object, not an array — one suggested fix, matching one Fix… on the
Problems row.

| field | type | rule |
|---|---|---|
| `message` | string | Required. The `= help:` text (`wrap it in math: \\(\\tilde{c}_t\\)`). No `= help:` prefix. |
| `replacement` | object | Optional. Omitted when the help is advice only. |

`replacement`:

| field | type | rule |
|---|---|---|
| `source` | `{path, start_byte, end_byte}` | Same meaning as diagnostic `source` and `labels[].source`. `path` is a project-relative document in the request; it **need not** equal the diagnostic's `source.path` (a replacement may edit an included file). Offsets are UTF-8, zero-based, end-exclusive. `start_byte == end_byte` is an insertion. |
| `text` | string | The replacement (may be empty = delete). |

If the diagnostic's `source` is null, `replacement` may still be present
when it names its own `source.path`. A producer that cannot name a
document omits `replacement`.

Advice-only help (`message` without `replacement`) is valid. The Mac maps that
to today's `QuickFix.Refusal.noEdits` ("this suggestion is advice only").

## 4. `message` and `recovery` do not change meaning

- `message` is one line of human prose. It does not include `(recovery: …)`,
  label captions, notes, or help. Those have fields.
- `recovery` remains null or a short description of what was typeset instead.
  The CLI already prints it as `= recovery:` in full mode and as
  `(recovery: …)` in short mode.
- `code` and `suggestion` are unchanged from issue #76. Classify by `code`, not
  by `message` wording.

## 5. Producers

| producer | emits `labels` / `notes` / `help`? |
|---|---|
| `crates/compiler` `Diagnostic::to_json` / `to_json_with_paths` (runtime-v1 `compile_result.payload.diagnostics`) | **Yes**, this lane (#277 (1)+(3)). Empty by default; JSON keys only when non-empty. |
| `crates/compiler` protocol validation errors (bad path, no documents, …) | No. Those sites stay `{severity, message, source: null, recovery: null}` with no `code`. |
| `crates/render-pipeline` `display::Diagnostic::from_compiler` | **Not yet.** Today it copies `message`, severity, one span, and `recovery`, and hardcodes `code: "compiler"`. It does not forward `code`, `suggestion`, or these fields. Out of this lane (another crate; needs a `vendor/compiler` re-pin). |
| `flashtex check` / `crates/flashtex-cli` | Consumer, not a producer. It currently builds its `Diagnostic` from the pipeline display list, so it will not see the new fields until the row above lands. Direct `compile_result` JSON from the compiler binary does. |
| `apps/mac` | Consumer. `RuntimeV1.Diagnostic` today decodes only `severity`, `message`, `source`, `recovery`. Swift `Codable` ignores unknown keys, so old builds stay correct. |
| Other crates' `Diagnostic` types (paragraph-layout, project-index, vector-graphics, …) | Not this wire format. |

Issue #275 (diagnostic codes through the render pipeline) is a different crate
and is not this proposal.

## 6. Consumers

### 6.1 Old consumers (no change)

Anything that reads only the frozen keys, or that ignores unknown object keys
(`scripts/check_runtime.py`, Swift `Codable`, the current Mac
`RuntimeV1.Diagnostic`, the current CLI `Diagnostic` struct), sees the same
bytes when the new fields are unset and skips them when they are set. No
migration flag, no capability name.

### 6.2 CLI renderer (PR #303, `crates/flashtex-cli/src/report.rs`)

`render_full` today prints:

```
error[compiler]: <message>
  --> notes.tex:91:38
   |
91 |     ... \(\tilde{c}_t=...
   |            ^^^^^^
   |
   = recovery: skipped the command
```

It already has wrapping at 80 columns, colour, and a caret under the diagnostic
span. It is designed to plug the new fields in without changing that skeleton:

- Each `labels[]` entry: carets under `label.source` (primary uses the error
  colour; secondary uses a distinct colour when colour is on) and `label.text`
  on the caret line, rustc-style (`^^^^^^ this command`).
- If `labels` is absent, keep today's uncaptioned caret on `source`.
- Each `notes[]` entry: `= note: <text>`, wrapped like `= recovery:`.
- `help.message`: `= help: <text>`, same wrapping.
- `help.replacement` is not printed as a diff in the first renderer; the
  message is the help. A later `--diagnostics=full` improvement may show the
  replacement.
- `--diagnostics=short` stays `Diagnostic::line_text()` (today's one-liner,
  including `(recovery: …)`). New fields are not inlined into that line, so
  piped scripts and existing `tests/cli.rs` expectations stay byte-identical.
- `--diagnostics=json` stays the existing JSON report.

Until the pipeline forwards the fields, the CLI can only render them if it
starts reading compiler `compile_result` diagnostics directly. That forwarding
is not this lane.

### 6.3 Mac Problems panel (issue #277 part (4), `EditorDiagnostics`)

Out of this lane; specified so part (4) has a contract.

- **Display.** The Problems row and hover keep `message` as the title. Under it:
  each note, then `help.message` if present, then `recovery` as today. Secondary
  labels can become extra underlines in the source editor (same
  `SourceMapping` rebase as `Mark` today) or extra subtitle lines if the editor
  stays one-range-per-diagnostic.
- **Fix… / `EditorDiagnostics.QuickFix`.** Today's QuickFix prepares explanation-
  service suggestion edits (`{path, start_byte, end_byte, text}` into the
  compiled document) and rebases them onto the current text. `help.replacement`
  is the same shape (`replacement.source` supplies `path` / offsets):
  1. If `help.replacement` is present, `QuickFix.prepare` from that one edit.
     Refusal cases are unchanged (overlap with a user edit, bytes no longer
     match, not scalar-aligned, compiled text missing, `.otherDocument` until
     part (4) applies a cross-file fix).
  2. Else if `suggestion` is present, keep the existing did-you-mean
     replacement of the whole `source` range.
  3. Else the help is advice only → `.noEdits`.
- **Category.** `EditorDiagnostics` currently keys some categories off message
  text. Once it decodes `code`, switch to `code` (`unknown_command`,
  `unsupported_feature`, …). Unknown `code` values must be tolerated (issue #76).

The Mac decoder adding optional `labels` / `notes` / `help` / `code` /
`suggestion` on `RuntimeV1.Diagnostic` is part (4), not this proposal's
implementation.

## 7. Compatibility

- **Old producer, new consumer:** no new keys; consumer uses `source` as the
  only span, prints no notes/help, Fix… only from `suggestion` if that older
  field is present. Output looks like today.
- **New producer, old consumer:** extra keys ignored. `message` / `source` /
  `recovery` still describe the problem. No error, no corruption.
- **Unset fields:** omitted, so every existing fixture and every diagnostic that
  has not been rewritten stays byte-identical.
- **Pinned compile-result fixtures:** `protocol/fixtures/compile-result.json`
  has `"diagnostics":[]` and does not need to change.

## 8. What this lane will implement (and what it will not)

In `crates/compiler` only:

1. This proposal file (no frozen-contract edit; Commander can add a one-line
   pointer from `docs/contracts/runtime-v1.md` after this lands).
2. `Diagnostic` gains `labels`, `notes`, `help`, default empty; JSON emits them
   only when non-empty.
3. The first ~20 highest-frequency messages gain a real `help` (unsupported
   command naming package/mode, recognised-but-unimplemented package, missing
   `$` / `}` / `]`, script marker outside math, `\frac` requires math mode,
   undefined reference with nearest `\label` by edit distance, included file
   not found with paths tried, unimplemented environment, …). Message text
   changes stay minimal; every `tests/recovery.rs` expectation stays reachable.

Not in this lane:

- Editing `docs/contracts/**` or `protocol/*.schema.json`.
- `crates/flashtex-cli` (PR #303).
- `apps/mac` / `EditorDiagnostics` (part 4).
- `crates/render-pipeline` and `vendor/compiler` re-pin (needed before the CLI
  and the app's pipeline path see the fields).
- PR #305's `\maketitle` / `Parser::unsupported` text: this lane merges that
  branch if it still touches the same functions, then adds help on top.

## 9. Worked mapping of the issue's example

Owner-requested report:

```
error[E0201]: `\tilde` is not implemented in text mode
  --> notes.tex:91:38
   |
91 |     \item "... \(\tilde{c}_t=g(W_cz_t+b_c)\) ...
   |                     ^^^^^^ this command
   |
   = note: \tilde is a math accent; here it is outside math mode
   = help: wrap it in math: \(\tilde{c}_t\)
   = recovery: the command was skipped and its argument typeset as text
```

This proposal does **not** introduce rustc `E0xxx` codes. `code` stays the
issue-#76 snake_case set (`unsupported_feature` here). The CLI header is
`error[unsupported_feature]:` (or `error[compiler]:` until the pipeline
forwards `code`). Everything else in that report is `message` + one primary
label + one note + help + `recovery`.
